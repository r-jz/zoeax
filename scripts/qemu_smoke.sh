#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TIMEOUT_SECONDS=20
LOG_FILE="${LOG_FILE:-$ROOT_DIR/target/qemu-smoke.log}"
STRICT_SMOKE="${STRICT_SMOKE:-0}"

mkdir -p "$(dirname "$LOG_FILE")"

set +e
make -C "$ROOT_DIR" build >"$LOG_FILE" 2>&1
build_status=$?
if [[ $build_status -eq 0 ]]; then
    timeout "${TIMEOUT_SECONDS}s" make -C "$ROOT_DIR" run-qemu >>"$LOG_FILE" 2>&1
    status=$?
else
    status=$build_status
fi
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
    "[TEST][PASS] ROOTSERVER_MAPPED_SIMPLE"
    "[TEST][PASS] ROOTSERVER_RESUMED_SIMPLE"
    "[TEST][PASS] SIMPLE_MAIN"
    "[TEST][PASS] ROOTSERVER_DONE"
)

for marker in "${required_markers[@]}"; do
    if ! grep -Fq "$marker" "$LOG_FILE"; then
        echo "[FAIL] smoke marker not found: $marker"
        echo "log: $LOG_FILE"
        tail -n 80 "$LOG_FILE" || true
        exit 1
    fi
done

if grep -Fq "[TEST][FAIL]" "$LOG_FILE"; then
    echo "[FAIL] test failure marker detected"
    echo "log: $LOG_FILE"
    tail -n 80 "$LOG_FILE" || true
    exit 1
fi

pass_line="$(grep -n -F "[TEST][PASS] ROOTSERVER_DONE" "$LOG_FILE" | tail -n1 | cut -d: -f1)"
panic_line="$(grep -n -F "panicked at" "$LOG_FILE" | head -n1 | cut -d: -f1 || true)"

if [[ -n "$panic_line" && -n "$pass_line" && "$panic_line" -lt "$pass_line" ]]; then
    echo "[FAIL] panic detected before smoke completion marker"
    echo "log: $LOG_FILE"
    tail -n 80 "$LOG_FILE" || true
    exit 1
fi

if [[ "$STRICT_SMOKE" == "1" && -n "$panic_line" ]]; then
    echo "[FAIL] strict mode: panic detected in qemu log"
    echo "log: $LOG_FILE"
    tail -n 80 "$LOG_FILE" || true
    exit 1
fi

if [[ -n "$panic_line" ]]; then
    echo "[WARN] panic detected after completion marker (STRICT_SMOKE=0)"
fi

echo "[PASS] qemu smoke"
echo "log: $LOG_FILE"
