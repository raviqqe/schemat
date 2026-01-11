use crate::error::ApplicationError;
use alloc::rc::Rc;
use core::str::Utf8Error;
use glob::{Pattern, glob};
use std::path::{Path, PathBuf};

pub fn read_paths(
    base: &Path,
    paths: &[String],
    ignore_patterns: &[String],
) -> Result<impl Iterator<Item = PathBuf>, ApplicationError> {
    let ignore_patterns = Rc::new(
        ignore_patterns
            .iter()
            .map(|pattern| Pattern::new(&resolve_path(pattern, base).display().to_string()))
            .collect::<Result<Vec<_>, _>>()?,
    );
    let repository = gix::discover(base).ok();
    let repository_path = repository
        .as_ref()
        .and_then(|repository| repository.path().parent())
        .map(|path| resolve_path(path, base));

    Ok(paths
        .iter()
        .map(|path| resolve_path(path, base))
        .filter(|path| {
            repository_path
                .as_ref()
                .map(|parent| !path.starts_with(parent))
                .unwrap_or(true)
        })
        .map(|path| Ok(glob(&path.display().to_string())?.collect::<Result<Vec<_>, _>>()?))
        .collect::<Result<Vec<_>, ApplicationError>>()?
        .into_iter()
        .flatten()
        .filter({
            let ignore_patterns = ignore_patterns.clone();
            move |path| !match_patterns(path, &ignore_patterns)
        })
        .chain(
            (if let Some(repository) = repository {
                let index = repository.index_or_empty()?;
                let patterns = paths
                    .iter()
                    .map(|path| Pattern::new(path))
                    .collect::<Result<Vec<_>, _>>()?;

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
                            match_patterns(path, &patterns)
                                && !match_patterns(path, &ignore_patterns)
                        }),
                )
            } else {
                None
            })
            .into_iter()
            .flatten(),
        ))
}

fn match_patterns(path: &Path, patterns: &[Pattern]) -> bool {
    patterns.iter().any(|pattern| pattern.matches_path(path))
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

        let path = directory.path().join("foo");
        fs::write(&path, "").unwrap();

        let paths = read_paths(directory.path(), &["foo".into()], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
    }

    #[test]
    fn list_file_outside_directory() {
        let directory = tempdir().unwrap();

        let path = directory.path().join("foo");
        fs::write(&path, "").unwrap();

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

        let path = directory.path().join("foo");
        fs::write(&path, "").unwrap();

        let repository_directory = directory.path().join("bar");
        fs::create_dir_all(&repository_directory).unwrap();

        gix::init(&repository_directory).unwrap();

        let paths = read_paths(&repository_directory, &["../foo".into()], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
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
