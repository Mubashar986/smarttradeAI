mod llm_bridge;
mod middleware;
mod mql5_extractor;
mod routes;
mod state;

// Re-export the public API consumed by the c2-engine binary crate.
pub use routes::turns::run_turn_worker;
pub use state::AppState;

// Re-export types so that `super::TypeName` paths in the test module (and any
// downstream crate code that relied on `server::TypeName`) continue to resolve.
pub use state::{
    CreateSessionResponse, DeleteStrategyResponse, ListSessionsResponse, ListStrategiesResponse,
    SendMessageRequest, SessionDetailsResponse, StrategyDetailsResponse, SubmitTurnRequest,
    SubmitTurnResponse, TaskResultType, TaskStatus, TaskStatusResponse, TurnContext,
    TurnMessageType, UpdateStrategyRequest,
};

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use routes::health::{health, ready};
use routes::sessions::{
    create_session, get_session, list_sessions, stream_session_events, stream_session_websocket,
};
use routes::strategies::{delete_strategy, get_strategy, list_strategies, patch_strategy};
use routes::turns::{get_task, send_message, send_turn};

#[must_use]
pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Canonical frontend-facing API surface.
    let protected_v1 = Router::new()
        .route("/v1/sessions", post(create_session).get(list_sessions))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/turn", post(send_turn))
        .route("/v1/sessions/{id}/events", get(stream_session_events))
        .route("/v1/ws/{id}", get(stream_session_websocket))
        .route("/v1/tasks/{task_id}", get(get_task))
        .route("/v1/strategies", get(list_strategies))
        .route(
            "/v1/strategies/{id}",
            get(get_strategy).patch(patch_strategy).delete(delete_strategy),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            middleware::auth::JwtAuthConfig::from_env(),
            middleware::auth::require_jwt,
        ));

    // Legacy compatibility/debug routes. Keep behavior stable until callers migrate to /v1.
    Router::new()
        .route("/health", get(health))
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/sessions", post(create_session).get(list_sessions))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/events", get(stream_session_events))
        .route("/sessions/{id}/message", post(send_message))
        .merge(protected_v1)
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use crate::middleware::auth::AuthClaims;
    use super::{
        app, run_turn_worker, AppState, CreateSessionResponse, DeleteStrategyResponse,
        ListSessionsResponse, ListStrategiesResponse, SessionDetailsResponse,
        StrategyDetailsResponse, SubmitTurnRequest, SubmitTurnResponse, TaskResultType,
        TaskStatus, TaskStatusResponse, TurnContext, TurnMessageType, UpdateStrategyRequest,
    };
    use jsonwebtoken::{encode, EncodingKey, Header};
    use reqwest::Client;
    use std::fs;
    use std::net::SocketAddr;
    use std::sync::{Mutex as StdMutex, OnceLock};
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio::time::{sleep, timeout};

    struct TestServer {
        address: SocketAddr,
        handle: JoinHandle<()>,
        worker_handle: Option<JoinHandle<()>>,
    }

    impl TestServer {
        async fn spawn() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("test listener should bind");
            let address = listener
                .local_addr()
                .expect("listener should report local address");
            let handle = tokio::spawn(async move {
                axum::serve(listener, app(AppState::default()))
                    .await
                    .expect("server should run");
            });

            Self {
                address,
                handle,
                worker_handle: None,
            }
        }

        async fn spawn_with_worker() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("test listener should bind");
            let address = listener
                .local_addr()
                .expect("listener should report local address");
            let (state, turn_rx) = AppState::new(None);
            let worker_state = state.clone();
            let worker_handle = tokio::spawn(async move {
                run_turn_worker(worker_state, turn_rx).await;
            });
            let handle = tokio::spawn(async move {
                axum::serve(listener, app(state))
                    .await
                    .expect("server should run");
            });

            Self {
                address,
                handle,
                worker_handle: Some(worker_handle),
            }
        }

        fn url(&self, path: &str) -> String {
            format!("http://{}{}", self.address, path)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.handle.abort();
            if let Some(worker_handle) = &self.worker_handle {
                worker_handle.abort();
            }
        }
    }

    async fn create_session(client: &Client, server: &TestServer) -> CreateSessionResponse {
        client
            .post(server.url("/v1/sessions"))
            .send()
            .await
            .expect("create request should succeed")
            .error_for_status()
            .expect("create request should return success")
            .json::<CreateSessionResponse>()
            .await
            .expect("create response should parse")
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| StdMutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn bearer_token(secret: &str, user_id: &str) -> String {
        let claims = AuthClaims {
            sub: Some(user_id.to_string()),
            user_id: Some(user_id.to_string()),
            exp: 4_102_444_800,
            iat: Some(1_700_000_000),
            iss: None,
            aud: None,
        };
        format!(
            "Bearer {}",
            encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes())
            )
            .expect("jwt should encode")
        )
    }

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}", crate::state::unix_timestamp_millis()))
    }

    fn seed_local_strategy(
        strategies_dir: &std::path::Path,
        strategy_id: &str,
        user_id: &str,
        status: &str,
    ) {
        fs::create_dir_all(strategies_dir).expect("strategies dir should exist");
        let stem = format!("{}_seed", strategy_id);
        fs::write(
            strategies_dir.join(format!("{stem}.mq5")),
            "void OnTick() {}",
        )
        .expect("code file should write");
        fs::write(
            strategies_dir.join(format!("{stem}.json")),
            serde_json::to_string_pretty(&serde_json::json!({
                "strategy_id": strategy_id,
                "strategy_name": format!("Strategy {strategy_id}"),
                "status": status,
                "session_id": "session-1",
                "user_id": user_id,
                "pair": "EURUSD",
                "timeframe": "H1",
                "explanation": "seeded",
                "created_at": "unix:1",
                "updated_at": "unix:2"
            }))
            .expect("seed metadata should serialize"),
        )
        .expect("metadata should write");
    }

    async fn next_sse_frame(response: &mut reqwest::Response, buffer: &mut String) -> String {
        loop {
            if let Some(index) = buffer.find("\n\n") {
                let frame = buffer[..index].to_string();
                let remainder = buffer[index + 2..].to_string();
                *buffer = remainder;
                return frame;
            }

            let next_chunk = timeout(Duration::from_secs(5), response.chunk())
                .await
                .expect("SSE stream should yield within timeout")
                .expect("SSE stream should remain readable")
                .expect("SSE stream should stay open");
            buffer.push_str(&String::from_utf8_lossy(&next_chunk));
        }
    }

    #[tokio::test]
    async fn creates_lists_and_gets_v1_sessions() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        // given
        let created = create_session(&client, &server).await;

        // when
        let sessions = client
            .get(server.url("/v1/sessions"))
            .send()
            .await
            .expect("list request should succeed")
            .error_for_status()
            .expect("list request should return success")
            .json::<ListSessionsResponse>()
            .await
            .expect("list response should parse");
        let details = client
            .get(server.url(&format!("/v1/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("details response should parse");

        // then
        assert_eq!(created.session_id, "session-1");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].id, created.session_id);
        assert_eq!(sessions.sessions[0].message_count, 0);
        assert_eq!(details.id, "session-1");
        assert!(details.session.messages.is_empty());
    }

    #[tokio::test]
    async fn legacy_session_routes_remain_available_for_compatibility() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;

        let sessions = client
            .get(server.url("/sessions"))
            .send()
            .await
            .expect("legacy list request should succeed")
            .error_for_status()
            .expect("legacy list request should return success")
            .json::<ListSessionsResponse>()
            .await
            .expect("legacy list response should parse");
        let details = client
            .get(server.url(&format!("/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("legacy details request should succeed")
            .error_for_status()
            .expect("legacy details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("legacy details response should parse");

        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].id, created.session_id);
        assert_eq!(details.id, created.session_id);
    }

    #[tokio::test]
    async fn accepts_v1_turn_and_exposes_task_status() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;

        let accepted = client
            .post(server.url(&format!("/v1/sessions/{}/turn", created.session_id)))
            .json(&SubmitTurnRequest {
                message_type: TurnMessageType::Intent,
                text: "buy EURUSD when 50 SMA crosses above 200 SMA".to_string(),
                context: TurnContext::default(),
            })
            .send()
            .await
            .expect("turn request should succeed")
            .error_for_status()
            .expect("turn request should return success")
            .json::<SubmitTurnResponse>()
            .await
            .expect("turn response should parse");

        let task = client
            .get(server.url(&format!("/v1/tasks/{}", accepted.task_id)))
            .send()
            .await
            .expect("task request should succeed")
            .error_for_status()
            .expect("task request should return success")
            .json::<TaskStatusResponse>()
            .await
            .expect("task response should parse");

        assert_eq!(accepted.task_id, "task-1");
        assert_eq!(accepted.status, TaskStatus::Queued);
        assert_eq!(task.task_id, accepted.task_id);
        assert_eq!(task.status, TaskStatus::Queued);
        assert!(task.result_type.is_none());
        assert_eq!(task.payload["phase"], "queued");
    }

    #[tokio::test]
    async fn rejects_protected_v1_routes_without_bearer_token_when_jwt_is_enabled() {
        let _guard = env_lock();
        std::env::set_var("C2_JWT_SECRET", "test-secret");

        let server = TestServer::spawn().await;
        let client = Client::new();

        let response = client
            .post(server.url("/v1/sessions"))
            .send()
            .await
            .expect("request should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

        std::env::remove_var("C2_JWT_SECRET");
    }

    #[tokio::test]
    async fn allows_protected_v1_routes_with_valid_bearer_token_when_jwt_is_enabled() {
        let _guard = env_lock();
        let secret = "test-secret";
        std::env::set_var("C2_JWT_SECRET", secret);

        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = client
            .post(server.url("/v1/sessions"))
            .header("Authorization", bearer_token(secret, "user-1"))
            .send()
            .await
            .expect("request should complete")
            .error_for_status()
            .expect("request should be authorized")
            .json::<CreateSessionResponse>()
            .await
            .expect("response should parse");

        assert_eq!(created.session_id, "session-1");

        std::env::remove_var("C2_JWT_SECRET");
    }

    #[tokio::test]
    async fn lists_and_gets_local_strategies_for_resolved_user() {
        let _guard = env_lock();
        std::env::remove_var("C2_JWT_SECRET");
        let strategies_dir = temp_dir("server-local-strategies");
        std::env::set_var("STRATEGIES_DIR", &strategies_dir);

        seed_local_strategy(&strategies_dir, "local-one", "local-dev-user", "GENERATED");
        seed_local_strategy(&strategies_dir, "local-two", "other-user", "GENERATED");

        let server = TestServer::spawn().await;
        let client = Client::new();

        let list = client
            .get(server.url("/v1/strategies"))
            .send()
            .await
            .expect("list request should succeed")
            .error_for_status()
            .expect("list request should return success")
            .json::<ListStrategiesResponse>()
            .await
            .expect("list response should parse");

        assert_eq!(list.strategies.len(), 1);
        assert_eq!(list.strategies[0].id, "local-one");

        let details = client
            .get(server.url("/v1/strategies/local-one"))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<StrategyDetailsResponse>()
            .await
            .expect("details response should parse");

        assert_eq!(details.strategy.id, "local-one");
        assert_eq!(details.strategy.user_id, "local-dev-user");

        fs::remove_dir_all(&strategies_dir).expect("cleanup temp dir");
        std::env::remove_var("STRATEGIES_DIR");
    }

    #[tokio::test]
    async fn patches_and_soft_deletes_local_strategy() {
        let _guard = env_lock();
        std::env::remove_var("C2_JWT_SECRET");
        let strategies_dir = temp_dir("server-local-strategy-update");
        std::env::set_var("STRATEGIES_DIR", &strategies_dir);

        seed_local_strategy(&strategies_dir, "local-edit", "local-dev-user", "DRAFT");

        let server = TestServer::spawn().await;
        let client = Client::new();

        let patched = client
            .patch(server.url("/v1/strategies/local-edit"))
            .json(&UpdateStrategyRequest {
                status: Some("APPROVED".to_string()),
                explanation: Some("updated explanation".to_string()),
                ..UpdateStrategyRequest::default()
            })
            .send()
            .await
            .expect("patch request should succeed")
            .error_for_status()
            .expect("patch request should return success")
            .json::<StrategyDetailsResponse>()
            .await
            .expect("patch response should parse");

        assert_eq!(patched.strategy.status, "APPROVED");
        assert_eq!(patched.strategy.explanation, "updated explanation");

        let deleted = client
            .delete(server.url("/v1/strategies/local-edit"))
            .send()
            .await
            .expect("delete request should succeed")
            .error_for_status()
            .expect("delete request should return success")
            .json::<DeleteStrategyResponse>()
            .await
            .expect("delete response should parse");

        assert_eq!(deleted.strategy_id, "local-edit");
        assert_eq!(deleted.status, "DELETED");

        let missing = client
            .get(server.url("/v1/strategies/local-edit"))
            .send()
            .await
            .expect("follow-up request should succeed");
        assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);

        fs::remove_dir_all(&strategies_dir).expect("cleanup temp dir");
        std::env::remove_var("STRATEGIES_DIR");
    }

    #[tokio::test]
    async fn streams_message_events_and_persists_message_flow() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        // given
        let created = create_session(&client, &server).await;
        let mut response = client
            .get(server.url(&format!("/sessions/{}/events", created.session_id)))
            .send()
            .await
            .expect("events request should succeed")
            .error_for_status()
            .expect("events request should return success");
        let mut buffer = String::new();
        let snapshot_frame = next_sse_frame(&mut response, &mut buffer).await;

        // when
        let send_status = client
            .post(server.url(&format!("/sessions/{}/message", created.session_id)))
            .json(&super::SendMessageRequest {
                message: "hello from test".to_string(),
            })
            .send()
            .await
            .expect("message request should succeed")
            .status();
        let message_frame = next_sse_frame(&mut response, &mut buffer).await;
        let details = client
            .get(server.url(&format!("/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("details response should parse");

        // then
        assert_eq!(send_status, reqwest::StatusCode::NO_CONTENT);
        assert!(snapshot_frame.contains("event: snapshot"));
        assert!(snapshot_frame.contains("\"session_id\":\"session-1\""));
        assert!(message_frame.contains("event: message"));
        assert!(message_frame.contains("hello from test"));
        assert_eq!(details.session.messages.len(), 1);
        assert_eq!(
            details.session.messages[0],
            runtime::ConversationMessage::user_text("hello from test")
        );
    }

    #[tokio::test]
    async fn exposes_websocket_route_for_session_streaming() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;
        let response = client
            .get(server.url(&format!("/v1/ws/{}", created.session_id)))
            .send()
            .await
            .expect("websocket route request should succeed");

        assert!(
            response.status().is_client_error() || response.status().is_redirection(),
            "unexpected status for websocket handshake probe: {}",
            response.status()
        );
    }

    #[tokio::test]
    async fn worker_completes_incomplete_strategy_turn_with_clarification() {
        let server = TestServer::spawn_with_worker().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;
        let mut response = client
            .get(server.url(&format!("/v1/sessions/{}/events", created.session_id)))
            .send()
            .await
            .expect("events request should succeed")
            .error_for_status()
            .expect("events request should return success");
        let mut buffer = String::new();
        let _snapshot_frame = next_sse_frame(&mut response, &mut buffer).await;

        let accepted = client
            .post(server.url(&format!("/v1/sessions/{}/turn", created.session_id)))
            .json(&SubmitTurnRequest {
                message_type: TurnMessageType::Intent,
                text: "Create a simple SMA crossover strategy for EURUSD H1 with stop-loss 50 pips."
                    .to_string(),
                context: TurnContext::default(),
            })
            .send()
            .await
            .expect("turn request should succeed")
            .error_for_status()
            .expect("turn request should return success")
            .json::<SubmitTurnResponse>()
            .await
            .expect("turn response should parse");

        let mut saw_clarification = false;
        for _ in 0..8 {
            let frame = next_sse_frame(&mut response, &mut buffer).await;
            if frame.contains("event: clarification_question") {
                saw_clarification = true;
                break;
            }
        }

        let task = wait_for_task_completion(&client, &server, &accepted.task_id).await;

        assert!(saw_clarification, "expected clarification event");
        assert_eq!(task.result_type, Some(TaskResultType::Clarification));
        assert_eq!(task.payload["status"], "INCOMPLETE");
        assert_eq!(task.payload["missing_fields"][0], "action");
        assert_eq!(
            task.payload["next_question"],
            "What trading action? (BUY or SELL)"
        );
    }

    #[tokio::test]
    async fn worker_generates_code_and_validation_when_spec_is_complete() {
        let server = TestServer::spawn_with_worker().await;
        let client = Client::new();

        let created = create_session(&client, &server).await;
        let mut response = client
            .get(server.url(&format!("/v1/sessions/{}/events", created.session_id)))
            .send()
            .await
            .expect("events request should succeed")
            .error_for_status()
            .expect("events request should return success");
        let mut buffer = String::new();
        let _snapshot_frame = next_sse_frame(&mut response, &mut buffer).await;

        let accepted = client
            .post(server.url(&format!("/v1/sessions/{}/turn", created.session_id)))
            .json(&SubmitTurnRequest {
                message_type: TurnMessageType::Intent,
                text: "Build a BUY EURUSD H1 strategy when 50 SMA crosses above 200 SMA with reverse cross exit and stop-loss 50 pips."
                    .to_string(),
                context: TurnContext::default(),
            })
            .send()
            .await
            .expect("turn request should succeed")
            .error_for_status()
            .expect("turn request should return success")
            .json::<SubmitTurnResponse>()
            .await
            .expect("turn response should parse");

        let mut saw_generated_code = false;
        let mut saw_validation_feedback = false;
        for _ in 0..12 {
            let frame = next_sse_frame(&mut response, &mut buffer).await;
            if frame.contains("event: generated_code") {
                saw_generated_code = true;
            }
            if frame.contains("event: validation_feedback")
                && frame.contains("\"stage\":\"static_analysis\"")
            {
                saw_validation_feedback = true;
            }
            if saw_generated_code && saw_validation_feedback {
                break;
            }
        }

        let task = wait_for_task_completion(&client, &server, &accepted.task_id).await;

        assert!(saw_generated_code, "expected generated_code event");
        assert!(
            saw_validation_feedback,
            "expected static-analysis validation_feedback event"
        );
        assert_eq!(task.result_type, Some(TaskResultType::Generation));
        assert_eq!(task.payload["status"], "COMPLETE");
        assert_eq!(task.payload["analysis"]["passed"], true);
        assert_eq!(task.payload["ready_for_compile"], true);
        let generated_code = task.payload["generation"]["code"]
            .as_str()
            .expect("generated code should be a string");
        assert!(generated_code.contains("OnTick()"));
    }

    async fn wait_for_task_completion(
        client: &Client,
        server: &TestServer,
        task_id: &str,
    ) -> TaskStatusResponse {
        for _ in 0..20 {
            let task = client
                .get(server.url(&format!("/v1/tasks/{task_id}")))
                .send()
                .await
                .expect("task request should succeed")
                .error_for_status()
                .expect("task request should return success")
                .json::<TaskStatusResponse>()
                .await
                .expect("task response should parse");
            if task.status == TaskStatus::Completed || task.status == TaskStatus::Failed {
                return task;
            }
            sleep(Duration::from_millis(50)).await;
        }

        panic!("task `{task_id}` did not reach a terminal state in time");
    }
}
