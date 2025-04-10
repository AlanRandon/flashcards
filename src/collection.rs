use deserialize::Topic;
use itertools::Itertools;
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

pub mod deserialize;

#[derive(Debug)]
struct Document(Vec<deserialize::Card>);

#[allow(clippy::module_name_repetitions)]
pub enum DocumentCollection {
    Document(String),
    Collection {
        topic: Arc<str>,
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
                topic: name.to_string_lossy().to_string().into_boxed_str().into(),
                entries,
            })
        } else if path.extension() == Some(std::ffi::OsStr::new("toml")) {
            Ok(Self::Collection {
                topic: path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
                    .into_boxed_str()
                    .into(),
                entries: vec![Self::Document(fs::read_to_string(path)?)],
            })
        } else {
            Ok(Self::Empty)
        }
    }
}

impl DocumentCollection {
    fn into_cards(self, topics: &[Arc<Topic>]) -> Result<Vec<deserialize::Card>, toml::de::Error> {
        match self {
            DocumentCollection::Document(data) => Ok(Document::deserialize_with_topics(
                toml::de::Deserializer::new(&data),
                topics,
            )?
            .0),
            DocumentCollection::Collection { topic, entries } => {
                let topic = match topics.last() {
                    None => Arc::new(Topic::new(&topic)),
                    Some(last) => Arc::new(last.push(topic)),
                };

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

impl TryFrom<DocumentCollection> for Vec<deserialize::Card> {
    type Error = toml::de::Error;

    fn try_from(collection: DocumentCollection) -> Result<Vec<deserialize::Card>, Self::Error> {
        DocumentCollection::into_cards(collection, &[])
    }
}
