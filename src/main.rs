#![doc = include_str!("../README.md")]

extern crate alloc;

mod ast;
mod context;
mod error;
mod format;
mod parse;
mod position;
mod position_map;

use crate::{
    format::format,
    parse::{ParseError, parse, parse_comments, parse_hash_directives},
    position_map::PositionMap,
};
use alloc::rc::Rc;
use bumpalo::Bump;
use clap::Parser;
use colored::Colorize;
use core::error::Error;
use error::ApplicationError;
use futures::future::try_join_all;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};
use tokio::{
    fs::{read_to_string, write},
    io::{AsyncReadExt, AsyncWriteExt, stdin, stdout},
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
    /// Ignore a pattern.
    #[arg(short, long)]
    ignore: Vec<String>,
    /// Use a custom ignore file.
    #[arg(long)]
    ignore_file: Option<PathBuf>,
    /// Be verbose.
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run(Arguments::parse()).await {
        eprintln!("{error}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn run(
    Arguments {
        paths,
        check,
        ignore,
        ignore_file,
        verbose,
        ..
    }: Arguments,
) -> Result<(), Box<dyn Error>> {
    if paths.is_empty() && check {
        Err("cannot check stdin".into())
    } else if paths.is_empty() {
        format_stdin().await
    } else if check {
        check_paths(&paths, &ignore, ignore_file.as_deref(), verbose).await
    } else {
        format_paths(&paths, &ignore, ignore_file.as_deref(), verbose).await
    }
}

async fn check_paths(
    paths: &[String],
    ignore: &[String],
    ignore_file: Option<&Path>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let mut count = 0;
    let mut error_count = 0;

    for (result, path) in try_join_all(
        read_paths(paths, ignore, ignore_file)?
            .map(|path| spawn(async { (check_path(&path).await, path) })),
    )
    .await?
    {
        count += 1;

        match result {
            Ok(success) => {
                if !success {
                    eprintln!("{}\t{}", "FAIL".yellow(), path.display());
                    error_count += 1;
                } else if verbose {
                    eprintln!("{}\t{}", "OK".green(), path.display());
                }
            }
            Err(error) => {
                eprintln!("{}\t{}\t{}", "ERROR".red(), path.display(), error);
                error_count += 1;
            }
        }
    }

    if error_count == 0 {
        Ok(())
    } else {
        Err(format!("{error_count} / {count} file(s) failed").into())
    }
}

async fn format_paths(
    paths: &[String],
    ignore: &[String],
    ignore_file: Option<&Path>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let mut count = 0;
    let mut error_count = 0;

    for (result, path) in try_join_all(
        read_paths(paths, ignore, ignore_file)?
            .map(|path| spawn(async { (format_path(&path).await, path) })),
    )
    .await?
    {
        count += 1;

        match result {
            Ok(_) => {
                if verbose {
                    eprintln!("{}\t{}", "FORMAT".blue(), path.display());
                }
            }
            Err(error) => {
                eprintln!("{}\t{}\t{}", "ERROR".red(), path.display(), error);
                error_count += 1;
            }
        }
    }

    if error_count == 0 {
        Ok(())
    } else {
        Err(format!("{error_count} / {count} file(s) failed to format").into())
    }
}

fn read_paths(
    paths: &[String],
    ignore: &[String],
    ignore_file: Option<&Path>,
) -> Result<impl Iterator<Item = PathBuf>, ApplicationError> {
    let ignore = Rc::new(if ignore.is_empty() && ignore_file.is_none() {
        Gitignore::global().0
    } else {
        let mut builder = GitignoreBuilder::new(".");

        if let Some(file) = ignore_file {
            builder.add(file);
        }

        for pattern in ignore {
            builder.add_line(None, pattern)?;
        }

        builder.build()?
    });

    Ok(paths
        .iter()
        .map(|path| Ok::<_, ApplicationError>(glob::glob(path)?.collect::<Result<Vec<_>, _>>()?))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter({
            let ignore = ignore.clone();
            move |path| !ignore.matched_path_or_any_parents(path, false).is_ignore()
        }))
}

async fn format_stdin() -> Result<(), Box<dyn Error>> {
    let mut source = Default::default();
    stdin().read_to_string(&mut source).await?;
    let position_map = PositionMap::new(&source);
    let convert_error = |error| convert_parse_error(error, &source, &position_map);
    let allocator = Bump::new();

    let mut stdout = stdout();
    stdout
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
    stdout.flush().await?;

    Ok(())
}

async fn check_path(path: &Path) -> Result<bool, ApplicationError> {
    let source = read_to_string(path).await?;

    Ok(source == format_string(&source)?)
}

async fn format_path(path: &Path) -> Result<(), ApplicationError> {
    let source = read_to_string(path).await?;
    let formatted = format_string(&source)?;

    // Skip write to a file to improve performance and reduce workload to a file
    // system if the file is formatted already.
    if source != formatted {
        write(path, formatted).await?;
    }

    Ok(())
}

fn format_string(source: &str) -> Result<String, ApplicationError> {
    let position_map = PositionMap::new(source);
    let convert_error = |error: ParseError| convert_parse_error(error, source, &position_map);
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

fn convert_parse_error(
    error: ParseError,
    source: &str,
    position_map: &PositionMap,
) -> ApplicationError {
    ApplicationError::Parse(error.to_string(source, position_map))
}
