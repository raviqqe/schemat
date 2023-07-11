mod ast;
mod format;
mod parse;
mod position;

use crate::{format::format, parse::parse};
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

    println!(
        "{}",
        format(&parse(&source).map_err(|error| error.to_string())?)
    );

    Ok(())
}
