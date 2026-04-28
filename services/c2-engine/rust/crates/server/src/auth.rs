use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct JwtAuthConfig {
    secret: Option<String>,
    issuer: Option<String>,
    audience: Option<String>,
}

impl JwtAuthConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            secret: read_non_empty_env("C2_JWT_SECRET")
                .or_else(|| read_non_empty_env("JWT_SECRET")),
            issuer: read_non_empty_env("C2_JWT_ISSUER"),
            audience: read_non_empty_env("C2_JWT_AUDIENCE"),
        }
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.secret.is_some()
    }

    fn validation(&self) -> Validation {
        let mut validation = Validation::new(Algorithm::HS256);
        if let Some(issuer) = &self.issuer {
            validation.set_issuer(&[issuer]);
        }
        if let Some(audience) = &self.audience {
            validation.set_audience(&[audience]);
        }
        validation
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthClaims {
    pub sub: Option<String>,
    pub user_id: Option<String>,
    pub exp: usize,
    pub iat: Option<usize>,
    pub iss: Option<String>,
    pub aud: Option<String>,
}

impl AuthClaims {
    #[must_use]
    pub fn principal_id(&self) -> Option<&str> {
        self.user_id.as_deref().or(self.sub.as_deref())
    }
}

pub async fn require_jwt(
    State(config): State<JwtAuthConfig>,
    mut request: Request,
    next: Next,
) -> Response {
    if !config.is_enabled() {
        return next.run(request).await;
    }

    let Some(secret) = &config.secret else {
        return next.run(request).await;
    };

    let Some(header_value) = request.headers().get(header::AUTHORIZATION) else {
        return auth_error(StatusCode::UNAUTHORIZED, "missing bearer token");
    };

    let Ok(header_value) = header_value.to_str() else {
        return auth_error(StatusCode::UNAUTHORIZED, "invalid authorization header");
    };

    let Some(token) = header_value
        .strip_prefix("Bearer ")
        .or_else(|| header_value.strip_prefix("bearer "))
    else {
        return auth_error(StatusCode::UNAUTHORIZED, "expected bearer token");
    };

    let validation = config.validation();
    let token_data = match decode::<AuthClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    ) {
        Ok(token_data) => token_data,
        Err(error) => {
            return auth_error(
                StatusCode::UNAUTHORIZED,
                &format!("invalid jwt: {error}"),
            )
        }
    };

    if token_data.claims.principal_id().is_none() {
        return auth_error(
            StatusCode::UNAUTHORIZED,
            "jwt must contain `user_id` or `sub`",
        );
    }

    request.extensions_mut().insert(token_data.claims);
    next.run(request).await
}

fn auth_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        axum::Json(serde_json::json!({
            "error": message,
        })),
    )
        .into_response()
}

fn read_non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.trim().is_empty())
}
