#!/usr/bin/env sh
# Build every workspace crate.
set -eu

cd "$(dirname "$0")/.."
cargo build --workspace "$@"
