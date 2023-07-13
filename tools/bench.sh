#!/bin/sh

set -e

cargo install hyperfine

mkdir -p tmp
$(dirname $0)/generate_s_expressions.sh 5 >tmp/foo.scm

hyperfine 'schemat <tmp/foo.scm >/dev/null'
