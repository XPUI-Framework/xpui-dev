#!/usr/bin/env bash
#
# The cross-repository gate. The checks themselves are in `xtask/`, in Rust;
# this only starts them.
#
#   ./build-and-test.sh          everything, including each repository's gate
#   ./build-and-test.sh cross    only the cross-repository half — what CI runs

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
exec cargo run --quiet -p xtask -- "${1:-all}"
