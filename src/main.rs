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
use futures::future::try_join_all;
use std::{
    collections::BTreeSet,
    error::Error,
    io::{read_to_string, stdin},
    path::PathBuf,
    process::exit,
};
use tokio::task::spawn_blocking;

#[derive(clap::Parser)]
#[command(about, version)]
struct Arguments {
    /// Paths of files or directories to format or check the format of.
    #[arg()]
    paths: Vec<String>,
    /// Check if files are formatted correctly.
    #[arg(short, long)]
    check: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Arguments::parse()).await {
        eprintln!("{}", error);
        exit(1)
    }
}

async fn run(arguments: Arguments) -> Result<(), Box<dyn Error>> {
    if arguments.paths.is_empty() {
        return format_stdin();
    }

    let paths = try_join_all(
        arguments
            .paths
            .into_iter()
            .map(|path| spawn_blocking(move || glob::glob(&path))),
    )
    .await?
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .flatten()
    .collect::<Result<BTreeSet<_>, _>>()?;

    if arguments.check {
        check(&paths)
    } else {
        format_paths(&paths)
    }
}

fn format_stdin() -> Result<(), Box<dyn Error>> {
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

fn check(paths: &BTreeSet<PathBuf>) -> Result<(), Box<dyn Error>> {
    todo!()
}

fn format_paths(paths: &BTreeSet<PathBuf>) -> Result<(), Box<dyn Error>> {
    todo!()
}
