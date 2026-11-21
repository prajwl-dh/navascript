#!/usr/bin/env sh
# Run the full workspace test suite.
set -eu

cd "$(dirname "$0")/.."
cargo test --workspace "$@"
