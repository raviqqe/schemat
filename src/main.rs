mod ast;
mod format;
mod parse;

use crate::parse::parse;
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
    println!("{:?}", parse(&read_to_string(stdin())?));

    Ok(())
}
