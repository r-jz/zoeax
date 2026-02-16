# Testing Strategy

## Goals
- Detect regressions in kernel boot and capability/syscall behavior quickly.
- Keep local feedback fast while preserving a QEMU-based signal for real execution.
- Make failures easy to diagnose with stable log markers.

## Test Layers
1. Static checks (already in place)
- `cargo fmt --all`
- `cargo check`
- `cargo clippy -- -D warnings`

2. QEMU smoke (new)
- Command: `make test-smoke`
- Script: `scripts/qemu_smoke.sh`
- What it verifies:
  - Kernel initialization starts (`initialising kernel`)
  - Root process setup completed (`root process initialization finished`)
  - Kernel initialization completes (`initialization finished`)

3. QEMU syscall/cap regression (next step)
- Add dedicated rootserver test binaries (`rootserver/src/bin/test_*.rs`).
- Each binary should print deterministic markers:
  - `[TEST][PASS] <case>`
  - `[TEST][FAIL] <case>: <err>`
- Add a harness script that runs each case and matches markers.

## Case Matrix (Priority)
1. Capability creation and slot management
- Untyped retype success/failure.
- CNode copy/mint/move against empty and non-empty slots.

2. Address space operations
- Page/PageTable map/unmap success/failure.
- Alignment and duplicate mapping errors.

3. Thread and IPC operations
- TCB configure/write regs/resume.
- Endpoint send/recv sequencing.
- Notification wait/send wakeup behavior.

## CI Plan
1. Keep current jobs:
- `fmt`
- `check`
- `clippy`

2. Add smoke gate:
- Run `make test-smoke` with timeout.
- Archive QEMU log (`target/qemu-smoke.log`) as artifact on failure.

3. Add regression gate after test binaries exist:
- Run test harness across `test_*.rs`.
- Fail PR if any `[TEST][FAIL]` marker appears.

## Local Usage
- Fast dev checks:
  - `cargo check`
  - `cargo clippy -- -D warnings`
- Boot smoke:
  - `make test-smoke`
- Strict smoke (panic treated as failure):
  - `STRICT_SMOKE=1 make test-smoke`
