#![allow(missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use indoc::indoc;
use schemat::format_string;

const SMALL_SOURCE_SIZE: usize = 1024;
const LARGE_SOURCE_SIZE: usize = 100 * 1024;
const NESTING_DEPTH: usize = 64;

fn repeat(fragment: &str, size: usize) -> String {
    fragment.repeat(size.div_ceil(fragment.len()))
}

fn list(size: usize) -> String {
    repeat("(foo bar baz)\n", size)
}

fn nested_list(size: usize) -> String {
    repeat(
        &format!(
            "{}bar{}\n",
            "(foo\n".repeat(NESTING_DEPTH),
            ")".repeat(NESTING_DEPTH)
        ),
        size,
    )
}

fn quote(size: usize) -> String {
    repeat("'(foo `(bar ,baz ,@qux))\n", size)
}

fn string(size: usize) -> String {
    repeat("\"Hello, \\\"world\\\"!\\n\"\n", size)
}

fn comment(size: usize) -> String {
    repeat(
        indoc! {"
            ; foo
            (foo bar) ; baz
            #| qux |#
        "},
        size,
    )
}

fn definition(size: usize) -> String {
    repeat(
        indoc! {"
            (define (fibonacci x)
              (if (< x 2)
                x
                (+ (fibonacci (- x 1)) (fibonacci (- x 2)))))

            (define (sum xs)
              (let loop ((xs xs) (y 0))
                (if (null? xs)
                  y
                  (loop (cdr xs) (+ y (car xs))))))

        "},
        size,
    )
}

fn benchmark_format(criterion: &mut Criterion, name: &str, build_source: fn(usize) -> String) {
    for (size_name, size) in [("small", SMALL_SOURCE_SIZE), ("large", LARGE_SOURCE_SIZE)] {
        let source = build_source(size);

        criterion.bench_function(&format!("format_{name}_{size_name}"), |bencher| {
            bencher.iter(|| black_box(format_string(black_box(&source)).unwrap()))
        });
    }
}

fn format_source(criterion: &mut Criterion) {
    for (name, build_source) in [
        ("list", list as fn(usize) -> String),
        ("nested_list", nested_list),
        ("quote", quote),
        ("string", string),
        ("comment", comment),
        ("definition", definition),
    ] {
        benchmark_format(criterion, name, build_source);
    }
}

criterion_group!(benches, format_source);
criterion_main!(benches);
