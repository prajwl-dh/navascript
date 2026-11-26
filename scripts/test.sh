#!/usr/bin/env sh
# Run the full workspace test suite plus the compatibility harness unit tests.
set -eu

cd "$(dirname "$0")/.."
cargo test --workspace "$@"
python3 -m unittest discover -s tests/compatibility -p 'test_*.py'
