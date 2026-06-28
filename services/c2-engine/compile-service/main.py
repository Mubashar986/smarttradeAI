r"""
MQL5 Compile Service — FastAPI wrapper around MetaEditor 5 (metaeditor64.exe).

Runs on the Windows host. The Rust backend (in Docker) reaches this service
via host.docker.internal:8080/compile.

Environment variables:
    MT5_PATH            Path to MetaTrader 5 installation.
                        Default: C:\Program Files\MetaTrader 5

Input:  POST /compile  { "code": "...", "session_id": "..." }
Output: CompileResult JSON matching the Rust CompileResult struct.
"""

import base64
import logging
import os
import subprocess
import uuid
from typing import List, Optional

from fastapi import FastAPI
from pydantic import BaseModel

from parser import has_errors, parse_metaeditor_log

app = FastAPI(title="SmartTrade MQL5 Compile Service")

# Configure logging so we see everything in the console
logging.basicConfig(level=logging.DEBUG, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("compile-service")

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

DEFAULT_MT5_PATH = r"C:\Program Files\MetaTrader 5"
METAEDITOR_EXE = "metaeditor64.exe"
COMPILE_TIMEOUT_SECONDS = 30


def _mt5_path() -> str:
    """Return the MetaTrader 5 installation path from env or default."""
    return os.environ.get("MT5_PATH", DEFAULT_MT5_PATH)


def _metaeditor_path() -> str:
    """Return the full path to metaeditor64.exe."""
    return os.path.join(_mt5_path(), METAEDITOR_EXE)


def _experts_dir() -> str:
    """Return the directory where .mq5 files are written for compilation."""
    return os.path.join(_mt5_path(), "MQL5", "Experts", "smarttrade")


def _include_dir() -> str:
    """Return the MQL5 directory for the /inc flag (parent of Include)."""
    default_path = os.path.join(_mt5_path(), "MQL5")
    if os.path.isdir(os.path.join(default_path, "Include")):
        return default_path

    # Fallback: search in Roaming AppData
    appdata = os.environ.get("APPDATA")
    if appdata:
        terminal_dir = os.path.join(appdata, "MetaQuotes", "Terminal")
        if os.path.isdir(terminal_dir):
            try:
                for name in os.listdir(terminal_dir):
                    sub = os.path.join(terminal_dir, name, "MQL5")
                    if os.path.isdir(os.path.join(sub, "Include")):
                        return sub
            except OSError:
                pass
    return default_path


# ---------------------------------------------------------------------------
# Pydantic Models
# ---------------------------------------------------------------------------

class CompileRequest(BaseModel):
    code: str
    session_id: str


class CompileError(BaseModel):
    line: Optional[int] = None
    col: Optional[int] = None
    message: str
    error_type: str  # "error" or "warning"


class CompileResponse(BaseModel):
    success: bool
    status: Optional[str] = None
    retry: int = 1
    max_retries: int = 2
    errors: List[CompileError] = []
    warnings: List[CompileError] = []
    source: str = "metaeditor"
    note: Optional[str] = None
    message: Optional[str] = None
    ex5_base64: Optional[str] = None


# ---------------------------------------------------------------------------
# Routes
# ---------------------------------------------------------------------------

@app.get("/health")
def health() -> dict:
    """Health check. Returns whether metaeditor64.exe is found."""
    metaeditor = _metaeditor_path()
    found = os.path.isfile(metaeditor)
    return {
        "status": "ok" if found else "metaeditor_not_found",
        "metaeditor_found": found,
        "metaeditor_path": metaeditor,
    }


@app.post("/compile", response_model=CompileResponse)
def compile_endpoint(request: CompileRequest) -> CompileResponse:
    """
    Compile an MQL5 strategy using MetaEditor 5.

    Steps:
        1. Validate metaeditor64.exe exists.
        2. Create MT5_PATH/MQL5/Experts/smarttrade/ if missing.
        3. Write {session_id}.mq5 to disk.
        4. Run metaeditor64.exe /compile /log /inc.
        5. Parse UTF-16-LE compiler log.
        6. Check if {session_id}.ex5 was produced.
        7. Return structured result.
    """

    # Step 1: Validate MetaEditor exists.
    metaeditor = _metaeditor_path()
    if not os.path.isfile(metaeditor):
        return CompileResponse(
            success=False,
            status="METAEDITOR_NOT_FOUND",
            errors=[
                CompileError(
                    message=f"MetaEditor not found at {metaeditor}. "
                    f"Set MT5_PATH environment variable to your MetaTrader 5 installation.",
                    error_type="error",
                )
            ],
            source="metaeditor_not_found",
        )

    # Step 2: Create directory if missing.
    experts_dir = _experts_dir()
    try:
        os.makedirs(experts_dir, exist_ok=True)
    except OSError as e:
        return CompileResponse(
            success=False,
            status="DIRECTORY_CREATE_FAILED",
            errors=[
                CompileError(
                    message=f"Failed to create directory {experts_dir}: {e}",
                    error_type="error",
                )
            ],
            source="metaeditor",
        )

    # Step 3: Write .mq5 file.
    # Sanitize session_id for safe filename use.
    safe_id = _sanitize_filename(request.session_id)
    mq5_path = os.path.join(experts_dir, f"{safe_id}.mq5")
    ex5_path = os.path.join(experts_dir, f"{safe_id}.ex5")
    log_path = os.path.join(experts_dir, f"{safe_id}.log")

    # Clean up any previous .ex5 and .log to avoid false positives from old runs.
    for path_to_remove in [ex5_path, log_path]:
        if os.path.exists(path_to_remove):
            try:
                os.remove(path_to_remove)
            except OSError:
                pass

    try:
        with open(mq5_path, "w", encoding="utf-8") as f:
            f.write(request.code)
    except OSError as e:
        return CompileResponse(
            success=False,
            status="FILE_WRITE_FAILED",
            errors=[
                CompileError(
                    message=f"Failed to write {mq5_path}: {e}",
                    error_type="error",
                )
            ],
            source="metaeditor",
        )

    # Step 4: Run MetaEditor.
    # IMPORTANT: We build the command as a single STRING, not a list.
    # On Windows, subprocess.run(list) uses list2cmdline() which backslash-escapes
    # embedded double quotes. MetaEditor does NOT understand backslash-escaped quotes.
    # Passing a string sends the command line to CreateProcessW verbatim.
    include_dir = _include_dir()

    cmd_str = (
        f'"{metaeditor}" '
        f'/compile:"{mq5_path}" '
        f'/log:"{log_path}" '
        f'/inc:"{include_dir}"'
    )

    logger.info("=" * 60)
    logger.info("COMPILE REQUEST for session: %s", request.session_id)
    logger.info("Command: %s", cmd_str)
    logger.info("mq5_path exists: %s", os.path.isfile(mq5_path))
    logger.info("mq5_path: %s", mq5_path)
    logger.info("log_path: %s", log_path)
    logger.info("include_dir exists: %s", os.path.isdir(include_dir))
    logger.info("=" * 60)

    stderr_text = ""
    stdout_text = ""
    try:
        result = subprocess.run(
            cmd_str,               # <-- STRING, not list
            capture_output=True,
            text=False,            # Capture raw bytes
            timeout=COMPILE_TIMEOUT_SECONDS,
        )
        # Decode byte outputs safely
        stderr_text = result.stderr.decode("utf-8", errors="replace") if result.stderr else ""
        stdout_text = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
        returncode = result.returncode

        logger.info("MetaEditor returncode: %d", returncode)
        logger.info("MetaEditor stdout: %s", stdout_text[:500] if stdout_text else "(empty)")
        logger.info("MetaEditor stderr: %s", stderr_text[:500] if stderr_text else "(empty)")
        logger.info("log_path exists after run: %s", os.path.isfile(log_path))

        # List files in the experts_dir after compilation for diagnostics
        try:
            dir_contents = os.listdir(experts_dir)
            logger.info("Files in %s: %s", experts_dir, dir_contents)
        except OSError:
            logger.warning("Could not list experts_dir")

        # Check return code for explicit execution errors, but ONLY if we didn't get a log file or an ex5 file.
        # MetaEditor sometimes exits with code 1 even on successful compilation (e.g., if custom include paths don't exist).
        log_file_exists = os.path.isfile(log_path) and os.path.getsize(log_path) > 0
        ex5_exists = os.path.isfile(ex5_path)
        if returncode != 0 and not ex5_exists and not log_file_exists:
            return CompileResponse(
                success=False,
                status="METAEDITOR_EXIT_ERROR",
                errors=[
                    CompileError(
                        message=f"MetaEditor exited with code {returncode} and produced no output. "
                        f"stderr: {stderr_text[:500]}",
                        error_type="error",
                    )
                ],
                source="metaeditor",
                note=f"Command was: {cmd_str}",
            )
    except subprocess.TimeoutExpired:
        return CompileResponse(
            success=False,
            status="TIMEOUT",
            errors=[
                CompileError(
                    message=f"Compilation timed out after {COMPILE_TIMEOUT_SECONDS} seconds.",
                    error_type="error",
                )
            ],
            source="metaeditor",
            note="MetaEditor may be stuck or the code is too complex.",
        )
    except OSError as e:
        logger.error("Failed to launch MetaEditor: %s", e)
        return CompileResponse(
            success=False,
            status="SUBPROCESS_FAILED",
            errors=[
                CompileError(
                    message=f"Failed to launch MetaEditor: {e}",
                    error_type="error",
                )
            ],
            source="metaeditor",
        )

    # Step 5: Parse compiler log.
    # We check multiple possible log locations.
    # log_path is the explicit path we gave to MetaEditor via /log flag.
    possible_log_paths = [
        log_path,                                                  # explicit: what we told MetaEditor
        os.path.join(experts_dir, f"{safe_id}.log"),               # default: test-session-1.log
        os.path.join(experts_dir, f"{safe_id}.mq5.log"),           # alternative: test-session-1.mq5.log
    ]
    # Deduplicate (log_path and the first default are the same path)
    seen = set()
    unique_log_paths = []
    for p in possible_log_paths:
        norm = os.path.normpath(p)
        if norm not in seen:
            seen.add(norm)
            unique_log_paths.append(p)

    logger.info("Searching for log files in: %s", unique_log_paths)
    log_messages = []
    actual_log_path = None
    for lp in possible_log_paths:
        if os.path.exists(lp) and os.path.getsize(lp) > 0:
            log_messages = parse_metaeditor_log(lp)
            actual_log_path = lp
            break

    # Fallback: parse stderr if log is empty but stderr has content
    if not log_messages and stderr_text:
        for line in stderr_text.splitlines():
            line = line.strip()
            if not line:
                continue
            # Heuristic: any line containing "error" or "warning" might be a compiler message
            lower = line.lower()
            if "error" in lower or "warning" in lower:
                log_messages.append(
                    {
                        "line": None,
                        "col": None,
                        "message": line,
                        "error_type": "error" if "error" in lower else "warning",
                    }
                )

    errors = [m for m in log_messages if m["error_type"] == "error"]
    warnings = [m for m in log_messages if m["error_type"] == "warning"]

    # Step 6: Check if .ex5 was produced.
    ex5_exists = os.path.isfile(ex5_path)
    compile_success = ex5_exists and not has_errors(log_messages)

    if not compile_success:
        # Build error list.
        error_models = [
            CompileError(
                line=m.get("line"),
                col=m.get("col"),
                message=m["message"],
                error_type="error",
            )
            for m in errors
        ]
        warning_models = [
            CompileError(
                line=m.get("line"),
                col=m.get("col"),
                message=m["message"],
                error_type="warning",
            )
            for m in warnings
        ]

        status = "COMPILE_FAILED" if errors else "COMPILE_WARNING"
        note = None
        if not ex5_exists and not errors:
            log_info = f"Log checked: {actual_log_path or 'none found'}"
            stderr_info = f"stderr: {stderr_text[:200] if stderr_text else '(empty)'}"
            note = (
                f"No .ex5 file was produced and no errors were found in the log. "
                f"This may indicate the code is too minimal (missing OnInit/OnDeinit) "
                f"or a MetaEditor internal failure. {log_info}. {stderr_info}"
            )

        return CompileResponse(
            success=False,
            status=status,
            errors=error_models,
            warnings=warning_models,
            source="metaeditor",
            note=note,
            message=f"Compilation failed with {len(errors)} error(s) and {len(warnings)} warning(s).",
        )

    # Step 7: Success — read .ex5 and base64 encode.
    try:
        with open(ex5_path, "rb") as f:
            ex5_bytes = f.read()
        ex5_base64 = base64.b64encode(ex5_bytes).decode("ascii")
    except OSError as e:
        return CompileResponse(
            success=False,
            status="EX5_READ_FAILED",
            errors=[
                CompileError(
                    message=f"Compilation succeeded but failed to read .ex5 file: {e}",
                    error_type="error",
                )
            ],
            source="metaeditor",
        )

    warning_models = [
        CompileError(
            line=m.get("line"),
            col=m.get("col"),
            message=m["message"],
            error_type="warning",
        )
        for m in warnings
    ]

    return CompileResponse(
        success=True,
        status="COMPILED",
        errors=[],
        warnings=warning_models,
        source="metaeditor",
        message="Compilation successful.",
        ex5_base64=ex5_base64,
    )


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _sanitize_filename(name: str) -> str:
    """
    Sanitize a string for safe use as a filename.

    Replaces characters that are unsafe on Windows filesystems.
    """
    # Replace common unsafe characters with underscore
    unsafe = '<>:"/\\|?*'
    for ch in unsafe:
        name = name.replace(ch, "_")
    # Trim whitespace
    name = name.strip()
    # If empty after sanitization, use a UUID
    if not name:
        name = str(uuid.uuid4())
    return name


# ---------------------------------------------------------------------------
# Dev entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8080)
