#![feature(allocator_api)]

mod ast;
mod context;
mod format;
mod parse;
mod position;
mod position_map;

use crate::{
    format::format,
    parse::{parse, parse_comments, parse_hash_directives, ParseError},
    position_map::PositionMap,
};
use bumpalo::Bump;
use clap::Parser;
use std::{
    error::Error,
    io::{read_to_string, stdin},
    path::PathBuf,
    process::exit,
};

#[derive(clap::Parser)]
#[command(about, version)]
struct Arguments {
    /// Paths of files or directories to format or check the format of.
    #[arg()]
    paths: Vec<PathBuf>,
    /// Check if files are formatted correctly.
    #[arg(short, long)]
    check: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Arguments::parse()) {
        eprintln!("{}", error);
        exit(1)
    }
}

fn run(_arguments: Arguments) -> Result<(), Box<dyn Error>> {
    let source = read_to_string(stdin())?;
    let position_map = PositionMap::new(&source);
    let convert_error = |error: ParseError| error.to_string(&source, &position_map);
    let allocator = Bump::new();

    print!(
        "{}",
        format(
            &parse(&source, &allocator).map_err(convert_error)?,
            &parse_comments(&source, &allocator).map_err(convert_error)?,
            &parse_hash_directives(&source, &allocator).map_err(convert_error)?,
            &position_map,
            &allocator,
        )?
    );

    Ok(())
}
