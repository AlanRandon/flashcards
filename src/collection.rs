use crate::Card;
use itertools::Itertools;
use std::{fs, io, path::Path, sync::Arc};

mod deserialize;

#[derive(Debug)]
struct Document(Vec<Card>);

#[allow(clippy::module_name_repetitions)]
pub enum DocumentCollection {
    Document(String),
    Collection {
        topic: String,
        entries: Vec<DocumentCollection>,
    },
    RootCollection {
        entries: Vec<DocumentCollection>,
    },
    Empty,
}

impl DocumentCollection {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let entries = fs::read_dir(path)?
            .map(|entry| entry.and_then(|entry| Self::new_subdir(entry.path())))
            .try_collect()?;

        Ok(Self::RootCollection { entries })
    }

    pub fn new_subdir(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        let Some(name) = path.file_name() else {
            return Ok(Self::Empty);
        };

        if name.to_str() == Some(".git") {
            return Ok(Self::Empty);
        }

        if path.is_dir() {
            let entries = fs::read_dir(path)?
                .map(|entry| entry.and_then(|entry| Self::new_subdir(entry.path())))
                .try_collect()?;

            Ok(Self::Collection {
                topic: name.to_string_lossy().to_string(),
                entries,
            })
        } else {
            let Some(stem) = path.file_stem() else {
                return Ok(Self::Empty);
            };
            Ok(Self::Collection {
                topic: stem.to_string_lossy().to_string(),
                entries: vec![Self::Document(fs::read_to_string(path)?)],
            })
        }
    }
}

impl DocumentCollection {
    fn into_cards(self, topics: &[Arc<str>]) -> Result<Vec<Card>, toml::de::Error> {
        match self {
            DocumentCollection::Document(data) => {
                Ok(Document::deserialize_toml_with_topics(&data, topics)?.0)
            }
            DocumentCollection::Collection { topic, entries } => {
                let topic: Arc<str> = format!(
                    "{}{topic}",
                    match topics.last() {
                        None => String::new(),
                        Some(topic) => format!("{topic}/"),
                    }
                )
                .into_boxed_str()
                .into();

                let mut topics = topics.to_owned();
                topics.push(topic);

                entries
                    .into_iter()
                    .map(|entry| entry.into_cards(&topics))
                    .flatten_ok()
                    .try_collect()
            }
            DocumentCollection::Empty => Ok(Vec::new()),
            DocumentCollection::RootCollection { entries } => entries
                .into_iter()
                .map(|entry| entry.into_cards(topics))
                .flatten_ok()
                .try_collect(),
        }
    }
}

impl TryFrom<DocumentCollection> for Vec<Card> {
    type Error = toml::de::Error;

    fn try_from(collection: DocumentCollection) -> Result<Vec<Card>, Self::Error> {
        DocumentCollection::into_cards(collection, &[])
    }
}
