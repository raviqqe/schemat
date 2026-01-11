use crate::error::ApplicationError;
use alloc::rc::Rc;
use core::str::Utf8Error;
use glob::{Pattern, glob};
use std::path::{Path, PathBuf};

pub fn read_paths(
    base: &Path,
    paths: &[String],
    exclude_patterns: &[String],
) -> Result<impl Iterator<Item = PathBuf>, ApplicationError> {
    let exclude_patterns = Rc::new(compile_patterns(exclude_patterns, base)?);
    let repository = gix::discover(base).ok();
    let repository_directory = repository
        .as_ref()
        .and_then(|repository| repository.path().parent())
        .map(ToOwned::to_owned);

    Ok(paths
        .iter()
        .map(|path| resolve_path(path, base))
        .filter(|path| {
            repository_directory
                .as_ref()
                .map(|parent| !path.starts_with(parent))
                .unwrap_or(true)
        })
        .map(|path| {
            Ok(glob(&path.display().to_string())?
                .chain(glob(&path.join("**/*").display().to_string())?)
                .collect::<Result<Vec<_>, _>>()?)
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?
        .into_iter()
        .flatten()
        .filter({
            let exclude_patterns = exclude_patterns.clone();
            move |path| !path.is_dir() && !match_patterns(path, &exclude_patterns)
        })
        .chain(
            (if let Some(repository) = repository {
                let index = repository.index_or_empty()?;
                let patterns = compile_patterns(paths, base)?;

                Some(
                    index
                        .entries()
                        .iter()
                        .map(|entry| {
                            Ok(PathBuf::from(str::from_utf8(entry.path(&index).as_ref())?))
                        })
                        .collect::<Result<Vec<_>, Utf8Error>>()?
                        .into_iter()
                        .filter(move |path| {
                            let path = resolve_path(
                                path,
                                repository_directory
                                    .as_deref()
                                    .expect("repository directory"),
                            );

                            patterns.iter().any(|pattern| pattern.matches_path(&path))
                                && !match_patterns(&path, &exclude_patterns)
                        }),
                )
            } else {
                None
            })
            .into_iter()
            .flatten(),
        ))
}

fn compile_patterns(patterns: &[String], base: &Path) -> Result<Vec<Pattern>, glob::PatternError> {
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

pub fn display_path(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .map_or_else(|_| path.display(), |path| path.display())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn list_file() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo".into()], &[])
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

        let paths = read_paths(&bar_directory, &["../foo".into()], &[])
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

        let paths = read_paths(&repository_directory, &["../foo".into()], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
    }

    #[test]
    fn list_file_in_directory() {
        let directory = tempdir().unwrap();

        fs::create_dir_all(directory.path().join("foo")).unwrap();
        fs::write(directory.path().join("foo/foo"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo".into()], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo/foo")]);
    }

    #[test]
    fn list_file_in_current_directory() {
        let directory = tempdir().unwrap();

        fs::create_dir_all(directory.path().join("foo")).unwrap();
        fs::write(directory.path().join("foo/foo"), "").unwrap();
        fs::write(directory.path().join("bar"), "").unwrap();

        let paths = read_paths(directory.path(), &[".".into()], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(
            paths,
            [
                directory.path().join("bar"),
                directory.path().join("foo/foo")
            ]
        );
    }

    #[test]
    fn exclude_file() {
        let directory = tempdir().unwrap();

        fs::write(directory.path().join("foo"), "").unwrap();
        fs::write(directory.path().join("bar"), "").unwrap();

        let paths = read_paths(directory.path(), &["*".into()], &["foo".into()])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("bar")]);
    }

    #[test]
    fn exclude_directory() {
        let directory = tempdir().unwrap();

        fs::create_dir_all(directory.path().join("foo")).unwrap();
        fs::write(directory.path().join("foo/foo"), "").unwrap();

        let paths = read_paths(directory.path(), &["foo/foo".into()], &["foo".into()])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [] as [PathBuf; _]);
    }

    mod display {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn handle_current_directory() {
            assert_eq!(
                &display_path(&Path::new("foo"), &Path::new(".")),
                Path::new("foo")
            );
        }

        #[test]
        fn remove_base_directory() {
            assert_eq!(
                &display_path(&Path::new("foo/bar"), &Path::new("foo")),
                Path::new("bar")
            );
        }
    }
}
