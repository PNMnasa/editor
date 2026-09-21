#!/usr/bin/env bash
#
# Native bash/CI counterpart of the 4 CI steps in .github/workflows/ci.yml.
# Same behavior as scripts/ci.ps1. Run on Linux/macOS.
#
# Usage:
#   bash scripts/ci.sh
set -euo pipefail

last_label=""

trap 'echo "==> $last_label failed" >&2' ERR

run() {
    last_label="$1"
    shift
    echo "==> $last_label"
    "$@"
}

run "cargo fmt --check" cargo fmt --check
run "cargo clippy --all-targets -- -D warnings" cargo clippy --all-targets -- -D warnings
run "cargo test" cargo test
run "cargo build --release" cargo build --release

echo ""
echo "All 4 CI steps passed."