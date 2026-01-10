#!/bin/sh

set -e

cargo build --release

export PATH=$PWD/target/release:$PATH

go tool agoa
