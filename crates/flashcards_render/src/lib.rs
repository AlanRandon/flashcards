use std::collections::HashSet;
use std::fmt::Display;
use std::sync::Arc;

mod deserialize;
pub mod loader;
pub mod render;

#[derive(Debug, Default, serde::Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Markdown,
    Tex,
    Typst,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Tex => write!(f, "tex"),
            Self::Typst => write!(f, "typst"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Topic(pub Vec<Arc<str>>);

impl Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("/"))
    }
}

#[derive(Debug, Hash)]
pub struct Source {
    pub source: String,
    pub format: Format,
}

#[derive(Debug, Hash)]
pub struct Rendered {
    pub source: Source,
    pub html: String,
}

#[derive(Debug)]
pub struct Card<T> {
    pub term: T,
    pub definition: T,
    pub topics: HashSet<Arc<Topic>>,
}
