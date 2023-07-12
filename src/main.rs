mod ast;
mod context;
mod format;
mod parse;
mod position;
mod position_map;

use crate::{
    format::format,
    parse::{parse, parse_comments},
    position_map::PositionMap,
};
use std::{
    error::Error,
    io::{read_to_string, stdin},
    process::exit,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error);
        exit(1)
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let source = read_to_string(stdin())?;
    let position_map = PositionMap::new(&source);

    print!(
        "{}",
        format(
            &parse(&source).map_err(|error| error.to_string(&source, &position_map))?,
            &parse_comments(&source).map_err(|error| error.to_string(&source, &position_map))?,
            &position_map,
        )
    );

    Ok(())
}
