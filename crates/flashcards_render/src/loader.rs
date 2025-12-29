use itertools::Itertools;

use crate::{Card, Source, Topic, deserialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
#[error("failure to deserialize error")]
pub struct DeserializeError(#[from] toml::de::Error);

#[derive(Debug, thiserror::Error)]
#[error("failed to decode utf-8")]
pub struct Utf8Error;

#[derive(Debug, Clone)]
enum PathSegments {
    Root,
    Child {
        parent: Arc<PathSegments>,
        segment: Arc<str>,
    },
}

impl PathSegments {
    fn into_vec(&self) -> Vec<Arc<str>> {
        match self {
            PathSegments::Root => Vec::new(),
            PathSegments::Child {
                parent,
                segment: name,
            } => {
                let mut parent_segments = parent.into_vec();
                parent_segments.push(Arc::clone(name));
                parent_segments
            }
        }
    }
}

#[derive(Debug)]
pub struct FileContents {
    pub path: PathBuf,
    path_segments: Vec<Arc<str>>,
    content: String,
}

impl FileContents {
    pub fn into_cards(self) -> Result<Vec<Card<Source>>, toml::de::Error> {
        let mut cards = deserialize::parse(&self.content)?;
        let topics = Topic(self.path_segments)
            .ancestors()
            .map(Arc::new)
            .collect_vec();

        for card in cards.iter_mut() {
            card.topics.extend(topics.iter().cloned());
        }

        Ok(cards)
    }
}

pub fn load_dir(path: impl AsRef<Path>) -> impl Iterator<Item = std::io::Result<FileContents>> {
    type BoxedIter = Box<dyn Iterator<Item = std::io::Result<FileContents>>>;

    fn load_dir_inner(path: impl AsRef<Path>, parent: Arc<PathSegments>) -> BoxedIter {
        let entries = match std::fs::read_dir(&path) {
            Ok(entry) => entry,
            Err(err) => return Box::new(std::iter::once(Err(err))),
        };

        let files = entries.flat_map(move |entry| -> BoxedIter {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => return Box::new(std::iter::once(Err(err))),
            };

            let file_type = match entry.file_type() {
                Ok(entry) => entry,
                Err(err) => return Box::new(std::iter::once(Err(err))),
            };

            let file_name = entry.file_name();
            let file_name = match file_name.to_str() {
                Some(file_name) => file_name,
                None => {
                    return Box::new(std::iter::once(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        Utf8Error,
                    ))));
                }
            };
            let segment = Arc::from(
                file_name
                    .trim_end_matches(".toml")
                    .to_string()
                    .into_boxed_str(),
            );

            let segments = Arc::new(PathSegments::Child {
                parent: Arc::clone(&parent),
                segment,
            });

            if file_type.is_dir() {
                if file_name == ".git" {
                    return Box::new(std::iter::empty());
                }

                return load_dir_inner(entry.path(), segments);
            }

            if !file_name.ends_with(".toml") {
                return Box::new(std::iter::empty());
            }

            let path = entry.path();

            let content = match std::fs::read_to_string(&path) {
                Ok(entry) => entry,
                Err(err) => return Box::new(std::iter::once(Err(err))),
            };

            Box::new(std::iter::once(Ok(FileContents {
                path: path,
                path_segments: segments.into_vec(),
                content,
            })))
        });

        Box::new(files)
    }

    load_dir_inner(path, Arc::new(PathSegments::Root))
}
