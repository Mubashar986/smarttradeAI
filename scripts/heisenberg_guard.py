#!/usr/bin/env python3
"""Small, dependency-free policy guard for Heisenberg host adapters."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

CONTROL_PATHS = (
    ".heisenberg/", ".agents/", ".cursor/", ".claude/", ".gemini/",
    "academy/", "AGENTS.md", "CLAUDE.md", "scripts/heisenberg_guard.py",
    "skills/", "config/", "core/", "QUICKSTART.md",
)
WRITE_TOOL_NAMES = {
    "write_to_file", "replace_file_content", "multi_replace_file_content",
    "Write", "TabWrite", "edit_file", "Edit",
}


def read_json(path: Path) -> dict[str, Any] | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def workspace_from(payload: dict[str, Any], explicit: str | None) -> Path:
    if explicit:
        return Path(explicit).resolve()
    for key in ("workspacePaths", "workspace_roots"):
        paths = payload.get(key)
        if isinstance(paths, list) and paths:
            return Path(paths[0]).resolve()
    return Path(payload.get("cwd", ".")).resolve()


def policy_paths(workspace: Path) -> dict[str, Path]:
    root = workspace / ".heisenberg"
    return {
        "root": root,
        "policy": root / "policy.json",
        "skills": root / "skills.json",
        "ui_workflow": root / "ui-workflow.json",
        "tasks": root / "tasks",
        "artifacts": root / "artifacts",
        "receipts": root / "receipts",
    }


def all_manifests(paths: dict[str, Path]) -> list[tuple[Path, dict[str, Any]]]:
    manifests: list[tuple[Path, dict[str, Any]]] = []
    if not paths["tasks"].is_dir():
        return manifests
    for candidate in paths["tasks"].glob("*.json"):
        data = read_json(candidate)
        if data:
            manifests.append((candidate, data))
    return manifests


def active_manifests(paths: dict[str, Path]) -> list[tuple[Path, dict[str, Any]]]:
    return [
        item for item in all_manifests(paths)
        if item[1].get("status") in {"planned", "approved", "implementing", "verifying"}
    ]


def validate(workspace: Path, require_task: bool = False) -> tuple[bool, list[str], list[str]]:
    paths = policy_paths(workspace)
    errors: list[str] = []
    warnings: list[str] = []
    if not (workspace / "AGENTS.md").is_file():
        errors.append("AGENTS.md is missing")
    policy = read_json(paths["policy"])
    skills = read_json(paths["skills"])
    ui_workflow = read_json(paths["ui_workflow"])
    if policy is None:
        errors.append(".heisenberg/policy.json is missing or invalid JSON")
    if skills is None:
        errors.append(".heisenberg/skills.json is missing or invalid JSON")
    if ui_workflow is None:
        errors.append(".heisenberg/ui-workflow.json is missing or invalid JSON")
    if errors:
        return False, errors, warnings

    manifests = all_manifests(paths)
    if require_task and not manifests:
        errors.append("no active task manifest")
    for path, manifest in manifests:
        task_id = manifest.get("task_id")
        task_type = manifest.get("task_type")
        status = manifest.get("status")
        selected = manifest.get("selected_skills", [])
        required = manifest.get("required_artifacts", [])
        pre_edit = manifest.get("required_before_edit", [])
        if not task_id or not isinstance(selected, list) or not selected:
            errors.append(f"{path}: task_id and selected_skills are required")
            continue
        if not isinstance(pre_edit, list):
            errors.append(f"{path}: required_before_edit must be an array")
        known = (skills or {}).get("skills", {})
        required_sets = (skills or {}).get("required_skill_sets", {})
        if status not in {"planned", "approved", "implementing", "verifying", "complete", "blocked"}:
            errors.append(f"{path}: invalid status {status!r}")
        unknown = [skill for skill in selected if skill not in known]
        if unknown:
            errors.append(f"{path}: unknown skills: {', '.join(unknown)}")
        required_skills = required_sets.get(task_type, [])
        missing_skills = [skill for skill in required_skills if skill not in selected]
        if missing_skills:
            errors.append(f"{path}: task type {task_type!r} is missing required skills: {', '.join(missing_skills)}")
        missing_sources = [
            skill for skill in selected
            if skill in known and not (workspace / known[skill].get("source", "")).is_file()
        ]
        if missing_sources:
            errors.append(f"{path}: selected skill source is missing: {', '.join(missing_sources)}")

        ui_surface = manifest.get("ui_surface")
        if ui_surface:
            routes = (ui_workflow or {}).get("surface_routes", {})
            route = routes.get(ui_surface)
            if route is None:
                errors.append(f"{path}: unknown ui_surface {ui_surface!r}")
            else:
                missing_pre_edit = [
                    artifact for artifact in route.get("required_before_edit", [])
                    if artifact not in pre_edit
                ]
                if missing_pre_edit:
                    errors.append(f"{path}: ui route is missing pre-edit artifacts: {', '.join(missing_pre_edit)}")
                style_skill = manifest.get("style_skill")
                allowed_styles = route.get("style_selection", [])
                if style_skill and style_skill not in allowed_styles:
                    errors.append(f"{path}: style_skill {style_skill!r} is not compatible with {ui_surface}")
                if ui_surface == "native-mobile-ui" and manifest.get("platform") not in route.get("platforms", []):
                    errors.append(f"{path}: native-mobile-ui requires a declared supported platform")

        artifacts_to_check = []
        if status in {"approved", "implementing", "verifying", "complete"}:
            artifacts_to_check.extend(pre_edit)
        if status in {"verifying", "complete"}:
            artifacts_to_check.extend(required)
        for artifact in dict.fromkeys(artifacts_to_check):
            if not (paths["artifacts"] / task_id / artifact).is_file():
                errors.append(f"{path}: missing artifact {artifact}")
        receipt = paths["receipts"] / f"{task_id}.json"
        if manifest.get("status") == "complete" and not receipt.is_file():
            errors.append(f"{path}: complete task is missing receipt {receipt.name}")
    if not active_manifests(paths):
        warnings.append("no active task manifest; policy is in bootstrap mode")
    return not errors, errors, warnings


def target_path(payload: dict[str, Any], host: str) -> str:
    if host == "antigravity":
        args = payload.get("toolCall", {}).get("args", {})
        return str(args.get("TargetFile", ""))
    tool_input = payload.get("tool_input", {})
    return str(tool_input.get("path", tool_input.get("file_path", "")))


def is_control_path(path: str) -> bool:
    normalized = path.replace("\\", "/")
    return any(normalized.endswith(item) or f"/{item}" in normalized for item in CONTROL_PATHS)


def tool_name(payload: dict[str, Any], host: str) -> str:
    if host == "antigravity":
        return str(payload.get("toolCall", {}).get("name", ""))
    return str(payload.get("tool_name", ""))


def emit(host: str, event: str, allowed: bool, message: str = "") -> int:
    if host == "antigravity":
        if event == "pre-tool":
            print(json.dumps({"decision": "allow" if allowed else "deny", "reason": message}))
        elif event == "pre-invocation":
            print(json.dumps({"injectSteps": [{"ephemeralMessage": message}]}))
        elif event == "stop" and not allowed:
            print(json.dumps({"decision": "continue", "reason": message}))
        else:
            print("{}")
    elif host == "cursor":
        print(json.dumps({"permission": "allow" if allowed else "deny", "agent_message": message, "user_message": message}))
    else:
        print(message)
    return 0 if allowed else 1


def hook(args: argparse.Namespace) -> int:
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        payload = {}
    workspace = workspace_from(payload, args.workspace)
    paths = policy_paths(workspace)
    valid, errors, warnings = validate(workspace)
    if args.event in {"session", "pre-invocation"}:
        message = "Heisenberg policy active." if valid else "Heisenberg bootstrap required: " + "; ".join(errors)
        return emit(args.host, args.event, True, message)
    if args.event == "pre-tool":
        name = tool_name(payload, args.host)
        path = target_path(payload, args.host)
        if name not in WRITE_TOOL_NAMES or is_control_path(path):
            return emit(args.host, args.event, True)
        manifests = active_manifests(paths)
        if not manifests:
            return emit(args.host, args.event, False, "Create an active .heisenberg/tasks manifest before editing product code.")
        _, manifest = manifests[0]
        if manifest.get("status") not in {"approved", "implementing"}:
            return emit(args.host, args.event, False, "Task must be approved or implementing before editing product code.")
        task_id = manifest.get("task_id", "")
        missing = [
            artifact for artifact in manifest.get("required_before_edit", [])
            if not (paths["artifacts"] / task_id / artifact).is_file()
        ]
        if missing:
            return emit(args.host, args.event, False, "Create required pre-edit artifacts: " + ", ".join(missing))
        return emit(args.host, args.event, True)
    if args.event == "stop":
        complete = [item for item in all_manifests(paths) if item[1].get("status") == "complete"]
        if complete:
            valid, errors, _ = validate(workspace)
            return emit(args.host, args.event, valid, "; ".join(errors))
        return emit(args.host, args.event, True)
    return emit(args.host, args.event, True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Heisenberg workflow artifacts.")
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate_parser = subparsers.add_parser("validate")
    validate_parser.add_argument("--workspace", default=".")
    validate_parser.add_argument("--require-task", action="store_true")
    hook_parser = subparsers.add_parser("hook")
    hook_parser.add_argument("--host", choices=("antigravity", "cursor"), required=True)
    hook_parser.add_argument("--event", required=True)
    hook_parser.add_argument("--workspace")
    args = parser.parse_args()
    if args.command == "validate":
        valid, errors, warnings = validate(Path(args.workspace).resolve(), args.require_task)
        print(json.dumps({"valid": valid, "errors": errors, "warnings": warnings}, indent=2))
        return 0 if valid else 1
    return hook(args)


if __name__ == "__main__":
    raise SystemExit(main())
