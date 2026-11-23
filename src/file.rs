use crate::error::Error;
use alloc::rc::Rc;
use core::{fmt::Display, str::Utf8Error};
use glob::{Pattern, glob};
use itertools::Itertools;
use std::path::{Path, PathBuf};

/// Reads paths of existing files and files matching glob patterns.
pub fn read_paths(
    base: &Path,
    paths: &[&str],
    ignore_patterns: &[&str],
) -> Result<impl Iterator<Item = PathBuf>, Error> {
    let ignore_patterns = Rc::new(compile_patterns(ignore_patterns, base)?);
    let repository = gix::discover(base).ok();
    let repository_directory = repository
        .as_ref()
        .and_then(gix::Repository::workdir)
        .map(ToOwned::to_owned);
    let (paths, patterns) = paths
        .iter()
        .copied()
        .partition::<Vec<_>, _>(|path| resolve_path(path, base).is_file());

    Ok(patterns
        .iter()
        .map(|pattern| resolve_path(pattern, base))
        .filter(|pattern| {
            repository_directory
                .as_ref()
                .map(|parent| !pattern.starts_with(parent))
                .unwrap_or(true)
        })
        .map(|pattern| Ok(glob(&pattern.display().to_string())?.collect::<Result<Vec<_>, _>>()?))
        .collect::<Result<Vec<_>, Error>>()?
        .into_iter()
        .flatten()
        .chain(paths.into_iter().map(move |path| resolve_path(path, base)))
        .filter({
            let ignore_patterns = ignore_patterns.clone();

            move |path| !path.is_dir() && !match_patterns(path, &ignore_patterns)
        })
        .chain(
            (if let Some((repository, directory)) = repository.zip(repository_directory) {
                let index = repository.index_or_empty()?;
                let patterns = compile_patterns(&patterns, base)?;

                Some(
                    index
                        .entries()
                        .iter()
                        .map(|entry| {
                            Ok(resolve_path(
                                str::from_utf8(entry.path(&index).as_ref())?,
                                &directory,
                            ))
                        })
                        .collect::<Result<Vec<_>, Utf8Error>>()?
                        .into_iter()
                        .filter(move |path| {
                            patterns.iter().any(|pattern| pattern.matches_path(path))
                                && !match_patterns(path, &ignore_patterns)
                        }),
                )
            } else {
                None
            })
            .into_iter()
            .flatten(),
        )
        .unique())
}

fn compile_patterns(patterns: &[&str], base: &Path) -> Result<Vec<Pattern>, glob::PatternError> {
    patterns
        .iter()
        .map(|pattern| Pattern::new(&resolve_path(pattern, base).display().to_string()))
        .collect::<Result<Vec<_>, _>>()
}

fn match_patterns(path: &Path, patterns: &[Pattern]) -> bool {
    patterns
        .iter()
        .any(|pattern| path.ancestors().any(|path| pattern.matches_path(path)))
}

fn resolve_path(path: impl AsRef<Path>, base: &Path) -> PathBuf {
    path_clean::clean(base.join(path))
}

/// Displays a path relative to a base directory.
pub fn display_path(path: &Path, base: &Path) -> impl Display {
    path.strip_prefix(base).unwrap_or(path).display()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::tempdir;

    fn track_files(repository: &gix::Repository, paths: &[&str]) {
        let mut index = gix::index::File::from_state(
            gix::index::State::new(repository.object_hash()),
            repository.git_dir().join("index"),
        );

        for path in paths {
            index.dangerously_push_entry(
                Default::default(),
                repository.object_hash().null(),
                gix::index::entry::Flags::empty(),
                gix::index::entry::Mode::FILE,
                (*path).into(),
            );
        }

        index.sort_entries();
        index.write(Default::default()).unwrap();
    }

    #[test]
    fn list_file() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
    }

    #[test]
    fn list_file_outside_directory() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo"), "").unwrap();

        let bar_directory = directory.path().join("bar");
        fs::create_dir_all(&bar_directory).unwrap();

        let paths = read_paths(&bar_directory, &["../foo"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
    }

    #[test]
    fn list_file_outside_git_repository() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo"), "").unwrap();

        let repository_directory = directory.path().join("bar");
        fs::create_dir_all(&repository_directory).unwrap();

        gix::init(&repository_directory).unwrap();

        let paths = read_paths(&repository_directory, &["../foo"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
    }

    #[test]
    fn list_files_outside_git_repository() {
        let directory = tempdir().unwrap();
        let directory = directory.path().canonicalize().unwrap();

        fs::write(directory.join("foo"), "").unwrap();

        let repository_directory = directory.join("bar");
        fs::create_dir_all(&repository_directory).unwrap();

        gix::init(&repository_directory).unwrap();

        let paths = read_paths(&repository_directory, &["../*"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.join("foo")]);
    }

    #[test]
    fn list_untracked_file_in_git_repository() {
        let directory = tempdir().unwrap();
        let directory = directory.path().canonicalize().unwrap();

        gix::init(&directory).unwrap();
        fs::write(directory.join("foo"), "").unwrap();

        let paths = read_paths(&directory, &["foo"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.join("foo")]);
    }

    #[test]
    fn list_tracked_file() {
        let directory = tempdir().unwrap();
        let directory = directory.path().canonicalize().unwrap();

        fs::write(directory.join("foo"), "").unwrap();
        track_files(&gix::init(&directory).unwrap(), &["foo"]);

        let paths = read_paths(&directory, &["foo"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.join("foo")]);
    }

    #[test]
    fn list_tracked_files_in_subdirectory() {
        let directory = tempdir().unwrap();
        let directory = directory.path().canonicalize().unwrap();

        let bar_directory = directory.join("bar");
        fs::create_dir_all(&bar_directory).unwrap();
        fs::write(directory.join("foo"), "").unwrap();
        fs::write(directory.join("bar/baz"), "").unwrap();
        track_files(&gix::init(&directory).unwrap(), &["foo", "bar/baz"]);

        let paths = read_paths(&bar_directory, &["*"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.join("bar/baz")]);
    }

    #[test]
    fn list_tracked_file_outside_directory() {
        let directory = tempdir().unwrap();
        let directory = directory.path().canonicalize().unwrap();

        let bar_directory = directory.join("bar");
        fs::create_dir_all(&bar_directory).unwrap();
        fs::write(directory.join("foo"), "").unwrap();
        fs::write(directory.join("bar/foo"), "").unwrap();
        track_files(&gix::init(&directory).unwrap(), &["foo", "bar/foo"]);

        let paths = read_paths(&bar_directory, &["../foo"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.join("foo")]);
    }

    #[test]
    fn list_file_in_directory() {
        let directory = tempdir().unwrap();

        fs::create_dir_all(directory.path().join("foo")).unwrap();
        fs::write(directory.path().join("foo/foo"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [] as [PathBuf; _]);
    }

    #[test]
    fn list_files_in_current_directory() {
        let directory = tempdir().unwrap();

        fs::create_dir_all(directory.path().join("foo")).unwrap();
        fs::write(directory.path().join("foo/foo"), "").unwrap();
        fs::write(directory.path().join("bar"), "").unwrap();

        let paths = read_paths(directory.path(), &["."], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [] as [PathBuf; _]);
    }

    #[test]
    fn list_file_matched_by_two_arguments_once() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo", "*"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
    }

    #[test]
    fn list_file_with_glob_characters() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo[0]"), "").unwrap();
        fs::write(directory.path().join("foo0"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo[0]"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo[0]")]);
    }

    #[test]
    fn list_tracked_file_with_glob_characters() {
        let directory = tempdir().unwrap();
        let directory = directory.path().canonicalize().unwrap();

        fs::write(directory.join("foo[0]"), "").unwrap();
        fs::write(directory.join("foo0"), "").unwrap();
        track_files(&gix::init(&directory).unwrap(), &["foo[0]", "foo0"]);

        let paths = read_paths(&directory, &["foo[0]"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.join("foo[0]")]);
    }

    #[test]
    fn list_file_matched_by_character_class() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo0"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo[0]"], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo0")]);
    }

    #[test]
    fn ignore_file() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo"), "").unwrap();
        fs::write(directory.path().join("bar"), "").unwrap();

        let paths = read_paths(directory.path(), &["*"], &["foo"])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("bar")]);
    }

    #[test]
    fn ignore_directory() {
        let directory = tempdir().unwrap();

        fs::create_dir_all(directory.path().join("foo")).unwrap();
        fs::write(directory.path().join("foo/foo"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo/foo"], &["foo"])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [] as [PathBuf; _]);
    }

    #[test]
    fn ignore_tracked_file() {
        let directory = tempdir().unwrap();
        let directory = directory.path().canonicalize().unwrap();

        fs::write(directory.join("foo"), "").unwrap();
        fs::write(directory.join("bar"), "").unwrap();
        track_files(&gix::init(&directory).unwrap(), &["foo", "bar"]);

        let paths = read_paths(&directory, &["*"], &["foo"])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.join("bar")]);
    }

    mod display {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn handle_current_directory() {
            assert_eq!(
                display_path(Path::new("foo"), Path::new(".")).to_string(),
                "foo"
            );
        }

        #[test]
        fn remove_base_directory() {
            assert_eq!(
                display_path(Path::new("foo/bar"), Path::new("foo")).to_string(),
                "bar"
            );
        }
    }
}
