#!/bin/sh

set -e

cargo build

export PATH=$PWD/target/debug:$PATH

go tool agoa
