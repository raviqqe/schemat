#![feature(allocator_api, error_in_core, iter_intersperse)]

mod ast;
mod context;
mod error;
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
use error::ApplicationError;
use futures::future::try_join_all;
use std::{
    error::Error,
    path::{Path, PathBuf},
    process::ExitCode,
};
use tokio::{
    fs::{read_to_string, write},
    io::{stdin, stdout, AsyncReadExt, AsyncWriteExt},
    spawn,
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
        check_paths(&arguments.paths, arguments.verbose).await
    } else {
        format_paths(&arguments.paths, arguments.verbose).await
    }
}

async fn check_paths(paths: &[String], verbose: bool) -> Result<(), Box<dyn Error>> {
    let mut count = 0;
    let mut error_count = 0;

    for result in try_join_all(read_paths(paths)?.map(|path| {
        spawn(async {
            let success = check_path(&path).await?;
            Ok::<_, ApplicationError>((path, success))
        })
    }))
    .await?
    {
        count += 1;

        match result {
            Ok((path, path_success)) => {
                if !path_success {
                    eprintln!("{}\t{}", "FAIL".red(), path.display());
                    error_count += 1;
                } else if verbose {
                    eprintln!("{}\t{}", "OK".green(), path.display());
                }
            }
            Err(error) => {
                eprintln!("{}\t{}", "ERROR".red(), error);
                error_count += 1;
            }
        }
    }

    if error_count == 0 {
        Ok(())
    } else {
        Err(format!("{} / {} file(s) failed", error_count, count).into())
    }
}

async fn format_paths(paths: &[String], verbose: bool) -> Result<(), Box<dyn Error>> {
    let mut count = 0;
    let mut error_count = 0;

    for result in try_join_all(read_paths(paths)?.map(|path| {
        spawn(async {
            format_path(&path).await?;
            Ok::<_, ApplicationError>(path)
        })
    }))
    .await?
    {
        count += 1;

        match result {
            Ok(path) => {
                if verbose {
                    eprintln!("{}\t{}", "FORMAT".blue(), path.display());
                }
            }
            Err(error) => {
                eprintln!("{}\t{}", "ERROR".red(), error);
                error_count += 1;
            }
        }
    }

    if error_count == 0 {
        Ok(())
    } else {
        Err(format!("{} / {} file(s) failed to format", error_count, count).into())
    }
}

fn read_paths(paths: &[String]) -> Result<impl Iterator<Item = PathBuf>, Box<dyn Error>> {
    Ok(paths
        .iter()
        .map(|path| Ok::<_, Box<dyn Error>>(glob::glob(path)?.collect::<Result<Vec<_>, _>>()?))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten())
}

async fn format_stdin() -> Result<(), Box<dyn Error>> {
    let mut source = Default::default();
    stdin().read_to_string(&mut source).await?;
    let position_map = PositionMap::new(&source);
    let convert_error = |error: ParseError| error.to_string("<stdin>", &source, &position_map);
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

async fn check_path(path: &Path) -> Result<bool, ApplicationError> {
    let source = read_to_string(path).await?;

    Ok(source == format_string(&source, &path.display().to_string())?)
}

async fn format_path(path: &Path) -> Result<(), ApplicationError> {
    write(
        path,
        format_string(&read_to_string(path).await?, &path.display().to_string())?,
    )
    .await?;

    Ok(())
}

fn format_string(source: &str, name: &str) -> Result<String, ApplicationError> {
    let position_map = PositionMap::new(source);
    let convert_error =
        |error: ParseError| ApplicationError::Parse(error.to_string(name, source, &position_map));
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
