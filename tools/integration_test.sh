#!/bin/sh

set -e

bundler install
cargo build --release

export PATH=target/release:$PATH

bundler exec cucumber --publish-quiet
