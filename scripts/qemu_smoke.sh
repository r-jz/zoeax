#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-10}"
LOG_FILE="${LOG_FILE:-$ROOT_DIR/target/qemu-smoke.log}"
STRICT_SMOKE="${STRICT_SMOKE:-0}"

mkdir -p "$(dirname "$LOG_FILE")"

set +e
timeout "${TIMEOUT_SECONDS}s" make -C "$ROOT_DIR" run >"$LOG_FILE" 2>&1
status=$?
set -e

if [[ $status -ne 0 && $status -ne 124 ]]; then
    echo "[FAIL] qemu exited with status=$status"
    echo "log: $LOG_FILE"
    tail -n 80 "$LOG_FILE" || true
    exit 1
fi

required_markers=(
    "initialising kernel"
    "root process initialization finished"
    "initialization finished"
)

for marker in "${required_markers[@]}"; do
    if ! grep -Fq "$marker" "$LOG_FILE"; then
        echo "[FAIL] smoke marker not found: $marker"
        echo "log: $LOG_FILE"
        tail -n 80 "$LOG_FILE" || true
        exit 1
    fi
done

if [[ "$STRICT_SMOKE" == "1" ]] && grep -Fq "panicked at" "$LOG_FILE"; then
    echo "[FAIL] strict mode: panic detected in qemu log"
    echo "log: $LOG_FILE"
    tail -n 80 "$LOG_FILE" || true
    exit 1
fi

if grep -Fq "panicked at" "$LOG_FILE"; then
    echo "[WARN] panic detected in smoke log (STRICT_SMOKE=0)"
fi

echo "[PASS] qemu smoke"
echo "log: $LOG_FILE"
