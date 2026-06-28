"""
MetaEditor 5 compiler log parser.

MetaEditor writes its compile logs in UTF-16-LE encoding.
Each log line follows this format:

    filename(line,col) : error CODE: message
    filename(line,col) : warning CODE: message

Examples:
    test.mq5(15,10) : error C2146: syntax error
    test.mq5(3,1) : warning C4101: unreferenced local variable

This module reads the log file, decodes UTF-16-LE, and extracts
structured error and warning messages.
"""

import os
import re
from typing import List, Optional


# Regex for MetaEditor log lines.
# Handles multiple formats:
#   filename(line,col) : error CODE: message
#   filename(line) : error CODE: message
#   filename : error CODE: message
#   error CODE: message
_LOG_LINE_RE = re.compile(
    r"""
    ^                           # start of line
    (?:                         # optional filename group
        (?P<filename>[^\(]+?)   # filename (non-greedy, before paren or colon)
        (?:                     # optional line/col group
            \(                 # opening paren
            (?P<line>\d+)       # line number
            (?:                 # optional column
                ,               # comma
                (?P<col>\d+)    # column number
            )?                  # column is optional
            \)                 # closing paren
        )?                      # line/col group is optional
        \s*:\s*                # colon separator
    )?                          # filename group is optional
    (?P<error_type>error|warning)  # error or warning
    (?:\s+(?P<code>\w+))?      # optional error code
    \s*:\s*                    # colon separator
    (?P<message>.+)            # the message
    $                           # end of line
    """,
    re.VERBOSE | re.IGNORECASE,
)


def parse_metaeditor_log(log_path: str) -> List[dict]:
    """
    Read a MetaEditor compile log and extract structured errors and warnings.

    Args:
        log_path: Path to the log file (UTF-16-LE encoded).

    Returns:
        List of dicts, each with keys:
            - line (int)
            - col (int)
            - message (str)
            - error_type (str): "error" or "warning"
    """
    if not os.path.exists(log_path):
        return []

    raw_bytes = _read_log_bytes(log_path)
    if raw_bytes is None:
        return []

    text = _decode_utf16le(raw_bytes)
    if text is None:
        return []

    return _extract_messages(text)


def _read_log_bytes(log_path: str) -> Optional[bytes]:
    """Read raw bytes from the log file."""
    try:
        with open(log_path, "rb") as f:
            return f.read()
    except OSError:
        return None


def _decode_utf16le(raw_bytes: bytes) -> Optional[str]:
    """
    Decode raw bytes as UTF-16-LE.

    MetaEditor writes logs in UTF-16-LE. If decoding fails,
    try UTF-16 (with BOM detection) as a fallback.
    """
    # Try UTF-16-LE first (MetaEditor default)
    try:
        return raw_bytes.decode("utf-16-le")
    except (UnicodeDecodeError, ValueError):
        pass

    # Fallback: UTF-16 with BOM detection
    try:
        return raw_bytes.decode("utf-16")
    except (UnicodeDecodeError, ValueError):
        pass

    # Last resort: ignore errors, treat as latin-1 (no byte is invalid)
    try:
        return raw_bytes.decode("utf-16-le", errors="ignore")
    except (UnicodeDecodeError, ValueError):
        return None


def _extract_messages(text: str) -> List[dict]:
    """Extract error and warning messages from decoded log text."""
    results = []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        match = _LOG_LINE_RE.match(line)
        if match:
            line_num = match.group("line")
            col_num = match.group("col")
            results.append(
                {
                    "line": int(line_num) if line_num else None,
                    "col": int(col_num) if col_num else None,
                    "message": match.group("message").strip(),
                    "error_type": match.group("error_type").lower(),
                }
            )
    return results


def has_errors(messages: List[dict]) -> bool:
    """Return True if any message is an error (not just a warning)."""
    return any(msg["error_type"] == "error" for msg in messages)
