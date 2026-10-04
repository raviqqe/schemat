#![doc = include_str!("../README.md")]

use clap::Parser;
use colored::Colorize;
use core::error::Error;
use futures::future::try_join_all;
use schemat::{ApplicationError, display_path, format_string, read_paths};
use std::{env::current_dir, path::Path, process::ExitCode};
use tokio::{
    fs::{read_to_string, write},
    io::{AsyncReadExt, AsyncWriteExt, stdin, stdout},
    spawn,
};

#[derive(clap::Parser)]
#[command(about, version)]
struct Arguments {
    /// A glob pattern of files.
    #[arg()]
    path: Vec<String>,
    /// Check if files are formatted correctly.
    #[arg(short, long)]
    check: bool,
    /// Ignore a glob pattern of files and directories.
    #[arg(short, long)]
    ignore: Vec<String>,
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
        path,
        check,
        ignore,
        verbose,
        ..
    }: Arguments,
) -> Result<(), Box<dyn Error>> {
    if path.is_empty() && check {
        Err("cannot check stdin".into())
    } else if path.is_empty() {
        format_stdin().await
    } else if check {
        check_paths(&path, &ignore, verbose).await
    } else {
        format_paths(&path, &ignore, verbose).await
    }
}

async fn check_paths(
    paths: &[String],
    ignore_patterns: &[String],
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let directory = current_dir()?;
    let mut count = 0;
    let mut error_count = 0;

    for (result, path) in try_join_all(
        read_paths(&directory, paths, ignore_patterns)?
            .map(|path| spawn(async { (check_path(&path).await, path) })),
    )
    .await?
    {
        count += 1;

        match result {
            Ok(success) => {
                if !success {
                    eprintln!("{}\t{}", "FAIL".yellow(), display_path(&path, &directory));
                    error_count += 1;
                } else if verbose {
                    eprintln!("{}\t{}", "OK".green(), display_path(&path, &directory));
                }
            }
            Err(error) => {
                eprintln!(
                    "{}\t{}\t{}",
                    "ERROR".red(),
                    display_path(&path, &directory),
                    error
                );
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
    ignore_patterns: &[String],
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let directory = current_dir()?;
    let mut count = 0;
    let mut error_count = 0;

    for (result, path) in try_join_all(
        read_paths(&directory, paths, ignore_patterns)?
            .map(|path| spawn(async { (format_path(&path).await, path) })),
    )
    .await?
    {
        count += 1;

        match result {
            Ok(_) => {
                if verbose {
                    eprintln!("{}\t{}", "FORMAT".blue(), display_path(&path, &directory));
                }
            }
            Err(error) => {
                eprintln!(
                    "{}\t{}\t{}",
                    "ERROR".red(),
                    display_path(&path, &directory),
                    error
                );
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

async fn format_stdin() -> Result<(), Box<dyn Error>> {
    let mut source = Default::default();
    stdin().read_to_string(&mut source).await?;

    let mut stdout = stdout();
    stdout.write_all(format_string(&source)?.as_bytes()).await?;
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
