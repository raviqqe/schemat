#!/bin/sh

set -e

bundler install
cargo build --release

export PATH=$PWD:target/release:$PATH

bundler exec cucumber --publish-quiet
