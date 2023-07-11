mod ast;
mod parse;

use crate::parse::parse;
use std::error::Error;
use std::process::exit;
use std::{io::read_to_string, io::stdin};

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
