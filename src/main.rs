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
use colored::Colorize;
use futures::future::try_join_all;
use glob::{GlobError, PatternError};
use std::{
    error::Error,
    path::{Path, PathBuf},
    process::ExitCode,
};
use tokio::{
    fs::{read_to_string, write},
    io::{stdin, stdout, AsyncReadExt, AsyncWriteExt},
};

#[derive(clap::Parser)]
#[command(about, version)]
struct Arguments {
    /// Glob patterns of files to format or check the format of.
    #[arg()]
    paths: Vec<String>,
    /// Check if files are formatted correctly.
    #[arg(short, long)]
    check: bool,
    /// Be verbose.
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run(Arguments::parse()).await {
        eprintln!("{}", error);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn run(arguments: Arguments) -> Result<(), Box<dyn Error>> {
    if arguments.paths.is_empty() && arguments.check {
        Err("cannot check stdin".into())
    } else if arguments.paths.is_empty() {
        format_stdin().await
    } else if arguments.check {
        let mut count = 0;
        let mut error_count = 0;

        for (path, path_success) in try_join_all(read_paths(arguments.paths)?.map(|path| async {
            let path = path?;
            let success = check_path(&path).await?;
            Ok::<_, Box<dyn Error>>((path, success))
        }))
        .await?
        {
            count += 1;

            if !path_success {
                eprintln!("{}\t{}", "FAIL".red(), path.display());
                error_count += 1;
            } else if arguments.verbose {
                eprintln!("{}\t{}", "OK".green(), path.display());
            }
        }

        if error_count == 0 {
            Ok(())
        } else {
            Err(format!("{} / {} file(s) failed", error_count, count).into())
        }
    } else {
        try_join_all(read_paths(arguments.paths)?.map(|path| async { format_path(&path?).await }))
            .await?;

        Ok(())
    }
}

fn read_paths(
    paths: Vec<String>,
) -> Result<impl Iterator<Item = Result<PathBuf, GlobError>>, PatternError> {
    Ok(paths
        .into_iter()
        .map(|path| glob::glob(&path))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten())
}

async fn format_stdin() -> Result<(), Box<dyn Error>> {
    let mut source = Default::default();
    stdin().read_to_string(&mut source).await?;
    let position_map = PositionMap::new(&source);
    let convert_error = |error: ParseError| error.to_string(&source, &position_map);
    let allocator = Bump::new();

    stdout()
        .write_all(
            format(
                &parse(&source, &allocator).map_err(convert_error)?,
                &parse_comments(&source, &allocator).map_err(convert_error)?,
                &parse_hash_directives(&source, &allocator).map_err(convert_error)?,
                &position_map,
                &allocator,
            )?
            .as_bytes(),
        )
        .await?;

    Ok(())
}

async fn check_path(path: &Path) -> Result<bool, Box<dyn Error>> {
    let source = read_to_string(path).await?;

    Ok(source == format_string(&source)?)
}

async fn format_path(path: &Path) -> Result<(), Box<dyn Error>> {
    write(path, format_string(&read_to_string(path).await?)?).await?;

    Ok(())
}

fn format_string(source: &str) -> Result<String, Box<dyn Error>> {
    let position_map = PositionMap::new(source);
    let convert_error = |error: ParseError| error.to_string(source, &position_map);
    let allocator = Bump::new();

    let source = format(
        &parse(source, &allocator).map_err(convert_error)?,
        &parse_comments(source, &allocator).map_err(convert_error)?,
        &parse_hash_directives(source, &allocator).map_err(convert_error)?,
        &position_map,
        &allocator,
    )?;

    Ok(source)
}
