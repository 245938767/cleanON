#!/usr/bin/env bash
set -euo pipefail

case "${RUNNER_OS:-$(uname -s)}" in
  Windows)
    export TMPDIR="${RUNNER_TEMP:-${TEMP:-$(pwd)/.tmp}}"
    ;;
  *)
    export TMPDIR="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
    ;;
esac

QA_TEMP_ROOT="$(mktemp -d)"
export QA_TEMP_ROOT
export FILE_ORGANIZER_AI_MODE="${FILE_ORGANIZER_AI_MODE:-mock}"
export FILE_ORGANIZER_DISABLE_CLOUD_AI="${FILE_ORGANIZER_DISABLE_CLOUD_AI:-1}"
export FILE_ORGANIZER_TEST_TEMP_ONLY="${FILE_ORGANIZER_TEST_TEMP_ONLY:-1}"

cleanup() {
  rm -rf "$QA_TEMP_ROOT"
}
trap cleanup EXIT

echo "Rust QA temp root: $QA_TEMP_ROOT"
echo "AI mode: $FILE_ORGANIZER_AI_MODE"

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
