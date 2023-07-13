#!/bin/sh

set -e

cargo install hyperfine
cargo build --release

mkdir -p tmp
$(dirname $0)/generate_s_expressions.sh 5 >tmp/foo.scm

hyperfine 'target/release/schemat <tmp/foo.scm >/dev/null'
