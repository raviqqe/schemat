#!/bin/sh

set -e

cargo install hyperfine

mkdir -p tmp
$(dirname $0)/generate_s_expressions.sh tmp/foo.scm
