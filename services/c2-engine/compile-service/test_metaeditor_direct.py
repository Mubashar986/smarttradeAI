import os
import subprocess
import sys

# --- Configuration ---
MT5_PATH = r"C:\Program Files\MetaTrader 5"
METAEDITOR = os.path.join(MT5_PATH, "metaeditor64.exe")
EXPERTS_DIR = os.path.join(MT5_PATH, "MQL5", "Experts", "smarttrade")
INCLUDE_DIR = os.path.join(MT5_PATH, "MQL5", "Include")

SESSION_ID = "diag-test"
MQ5_PATH = os.path.join(EXPERTS_DIR, f"{SESSION_ID}.mq5")
EX5_PATH = os.path.join(EXPERTS_DIR, f"{SESSION_ID}.ex5")
LOG_PATH = os.path.join(EXPERTS_DIR, f"{SESSION_ID}.log")

VALID_CODE = r"""
#property strict
#property copyright "SmartTradeAI Diagnostic"
#property version   "1.00"

int OnInit()
{
   return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
}

void OnTick()
{
   Comment("Diagnostic test");
}
"""

def main():
    print("=" * 60)
    print("MetaEditor Direct Diagnostic Test")
    print("=" * 60)

    # 1. Check MetaEditor exists
    print(f"\n[1] MetaEditor path: {METAEDITOR}")
    print(f"    Exists: {os.path.isfile(METAEDITOR)}")
    if not os.path.isfile(METAEDITOR):
        print("    ERROR: MetaEditor not found! Aborting.")
        sys.exit(1)

    # 2. Check experts dir
    print(f"\n[2] Experts dir: {EXPERTS_DIR}")
    print(f"    Exists: {os.path.isdir(EXPERTS_DIR)}")
    if not os.path.isdir(EXPERTS_DIR):
        print("    Creating directory...")
        os.makedirs(EXPERTS_DIR, exist_ok=True)

    # 3. Write .mq5 file
    print(f"\n[3] Writing .mq5 file: {MQ5_PATH}")
    try:
        with open(MQ5_PATH, "w", encoding="utf-8") as f:
            f.write(VALID_CODE)
        print(f"    Written: {os.path.getsize(MQ5_PATH)} bytes")
    except OSError as e:
        print(f"    ERROR writing file: {e}")
        print("    TIP: Try running this script as Administrator.")
        sys.exit(1)

    # 4. Clean old outputs
    for p in [EX5_PATH, LOG_PATH]:
        if os.path.exists(p):
            os.remove(p)
            print(f"    Cleaned: {p}")

    # 5. Build command STRING (not list!)
    cmd_str = (
        f'"{METAEDITOR}" '
        f'/compile:"{MQ5_PATH}" '
        f'/log:"{LOG_PATH}" '
        f'/inc:"{INCLUDE_DIR}"'
    )
    print(f"\n[4] Command (string):")
    print(f"    {cmd_str}")

    # 6. Run MetaEditor
    print(f"\n[5] Running MetaEditor...")
    result = subprocess.run(
        cmd_str,
        capture_output=True,
        text=False,
        timeout=30,
    )

    rc = result.returncode
    stdout = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    stderr = result.stderr.decode("utf-8", errors="replace") if result.stderr else ""

    print(f"    Return code: {rc}")
    print(f"    stdout: {stdout[:500] if stdout else '(empty)'}")
    print(f"    stderr: {stderr[:500] if stderr else '(empty)'}")

    # 7. Check results
    print(f"\n[6] Results:")
    print(f"    .ex5 exists: {os.path.isfile(EX5_PATH)}")
    print(f"    .log exists: {os.path.isfile(LOG_PATH)}")

    if os.path.isfile(EX5_PATH):
        print(f"    .ex5 size: {os.path.getsize(EX5_PATH)} bytes")
        print("    SUCCESS: COMPILATION SUCCEEDED!")
    else:
        print("    FAIL: NO .ex5 FILE -- compilation failed or MetaEditor did nothing.")

    if os.path.isfile(LOG_PATH):
        print(f"    .log size: {os.path.getsize(LOG_PATH)} bytes")
        # Try reading the log
        try:
            raw = open(LOG_PATH, "rb").read()
            # MetaEditor logs are usually UTF-16-LE
            try:
                text = raw.decode("utf-16-le")
            except:
                try:
                    text = raw.decode("utf-16")
                except:
                    text = raw.decode("latin-1")
            print(f"\n    --- Log contents ---")
            for line in text.splitlines()[:30]:
                line = line.strip()
                if line:
                    print(f"    {line}")
            print(f"    --- End log ---")
        except Exception as e:
            print(f"    Could not read log: {e}")
    else:
        print("    FAIL: NO .log FILE -- MetaEditor may not have received the /log flag correctly.")

    # 8. List directory
    print(f"\n[7] Files in {EXPERTS_DIR}:")
    try:
        for fn in sorted(os.listdir(EXPERTS_DIR)):
            fp = os.path.join(EXPERTS_DIR, fn)
            sz = os.path.getsize(fp) if os.path.isfile(fp) else "dir"
            print(f"    {fn:40s}  {sz}")
    except OSError as e:
        print(f"    Could not list: {e}")

    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
