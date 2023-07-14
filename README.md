# schemat

[![GitHub Action](https://img.shields.io/github/actions/workflow/status/raviqqe/schemat/test.yaml?branch=main&style=flat-square)](https://github.com/raviqqe/schemat/actions?query=workflow%3Atest)
[![Crate](https://img.shields.io/crates/v/schemat.svg?style=flat-square)](https://crates.io/crates/schemat)
[![License](https://img.shields.io/github/license/raviqqe/schemat.svg?style=flat-square)](https://github.com/raviqqe/schemat/blob/main/UNLICENSE)

Scheme formatter written in Rust

It supports:

- S-expressions
- Comments
- Quotes (e.g. `'`, `\``, and `,`)
- Hash directives
  - Shebang `#!/usr/bin/env gambit`
  - Language shorthand in Racket (e.g. `#lang racket`)

## Install

```sh
cargo install schemat
```

## Usage

```sh
schemat < in.scm > out.scm
```

## License

[The Unlicense](UNLICENSE)
