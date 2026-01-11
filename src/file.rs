use crate::error::ApplicationError;
use alloc::rc::Rc;
use core::str::Utf8Error;
use globwalk::GlobWalkerBuilder;
use ignore::gitignore::GitignoreBuilder;
use std::{
    io,
    path::{Path, PathBuf, absolute},
};

pub fn read_paths(
    directory: &Path,
    paths: &[String],
    ignored_patterns: &[String],
) -> Result<impl Iterator<Item = PathBuf>, ApplicationError> {
    let mut builder = GitignoreBuilder::new(directory);

    for pattern in ignored_patterns {
        builder.add_line(None, pattern)?;
    }

    let ignore = Rc::new(builder.build()?);
    let repository = gix::discover(directory).ok();
    let repository_path = repository
        .as_ref()
        .and_then(|repository| repository.path().parent())
        .map(resolve_path)
        .transpose()?;
    let paths = paths
        .iter()
        .map(|path| Ok((path, resolve_path(path)?)))
        .collect::<Result<Vec<_>, io::Error>>()?
        .iter()
        .filter(|(_, absolute_path)| {
            repository_path
                .as_ref()
                .map(|parent| !absolute_path.starts_with(parent))
                .unwrap_or(true)
        })
        .map(|(path, _)| path.to_string())
        .collect::<Vec<_>>();

    Ok(GlobWalkerBuilder::from_patterns(directory, &paths)
        .build()?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path().to_owned())
        .filter({
            let ignore = ignore.clone();
            move |path| !ignore.matched_path_or_any_parents(&path, false).is_ignore()
        })
        .chain(
            (if let Some(repository) = repository {
                let index = repository.index()?;
                let patterns = paths
                    .iter()
                    .map(|path| glob::Pattern::new(path))
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
                            patterns.iter().any(|pattern| pattern.matches_path(path))
                                && !ignore.matched_path_or_any_parents(path, false).is_ignore()
                        }),
                )
            } else {
                None
            })
            .into_iter()
            .flatten(),
        ))
}

fn resolve_path(path: impl AsRef<Path>) -> Result<PathBuf, io::Error> {
    Ok(path_clean::clean(absolute(path.as_ref())?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn read_file() {
        let directory = tempdir().unwrap();

        let path = directory.path().join("foo");
        fs::write(&path, "").unwrap();

        let paths = read_paths(directory.path(), &["foo".into()], &[])
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(paths, [directory.path().join("foo")]);
    }
}
