# schemat

[![GitHub Action](https://img.shields.io/github/actions/workflow/status/raviqqe/schemat/test.yaml?branch=main&style=flat-square)](https://github.com/raviqqe/schemat/actions?query=workflow%3Atest)
[![Crate](https://img.shields.io/crates/v/schemat.svg?style=flat-square)](https://crates.io/crates/schemat)
[![License](https://img.shields.io/github/license/raviqqe/schemat.svg?style=flat-square)](https://github.com/raviqqe/schemat/blob/main/UNLICENSE)

Scheme/S-expression formatter written in Rust

It supports:

- S-expressions
  - Parenthesis: `(` and `)`
  - Brackets: `[` and `]`
  - Braces: `{` and `}`
- Comments
- Quotes: `'`, `` ` ``, and `,`
- Hash directives
  - Shebang (e.g. `#!/usr/bin/env gsi`)
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
