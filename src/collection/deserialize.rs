use super::Document;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::fmt::Display;
use std::sync::Arc;
use toml::Table;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardFormat {
    #[default]
    Markdown,
    Tex,
}

#[derive(Debug)]
pub struct CardSide {
    pub text: String,
    pub format: CardFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(pub Vec<Arc<str>>);

impl Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("/"))
    }
}

impl Topic {
    pub fn new(topic: &str) -> Self {
        let segments = topic
            .split('/')
            .map(|segment| segment.to_string().into_boxed_str().into())
            .collect::<Vec<_>>();

        Self(segments)
    }

    pub fn push(&self, segment: Arc<str>) -> Self {
        let mut segments = self.0.clone();
        segments.push(segment);
        Self(segments)
    }

    pub fn basename(&self) -> &str {
        self.0.last().expect("topics have at least 1 component")
    }

    pub fn ancestors(&self) -> impl Iterator<Item = Self> {
        (1..=self.0.len()).map(|i| Self(self.0.get(0..i).unwrap().to_vec()))
    }
}

#[derive(Debug)]
pub struct Card {
    pub term: CardSide,
    pub definition: CardSide,
    pub topics: HashSet<Arc<Topic>>,
}

impl Document {
    pub fn deserialize_with_topics<'de, D>(
        deserializer: D,
        topics: &[Arc<Topic>],
    ) -> Result<Document, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut table = Table::deserialize(deserializer)?;
        let mut topics = topics.to_owned();

        topics.extend(
            table
                .remove("topics")
                .map_or(Ok(Vec::new()), Vec::<String>::deserialize)
                .map_err(Error::custom)?
                .iter()
                .map(|topic| Arc::new(Topic::new(topic))),
        );

        let mut cards = table
            .remove("cards")
            .map_or(Ok(Vec::new()), Vec::<Card>::deserialize)
            .map_err(Error::custom)?;

        for card in &mut cards {
            card.topics.extend(topics.iter().cloned());
            card.topics = card
                .topics
                .iter()
                .flat_map(|topics| topics.ancestors().collect::<Vec<_>>())
                .map(Arc::new)
                .collect();
        }

        Ok(Document(cards))
    }
}

impl<'de> Deserialize<'de> for Document {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize_with_topics(deserializer, &[])
    }
}

impl<'de> Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut table = Table::deserialize(deserializer)?;

        let Some(term) = table.remove("term") else {
            return Err(Error::missing_field("term"));
        };

        let Some(definition) = table.remove("definition") else {
            return Err(Error::missing_field("definition"));
        };

        let topics = table
            .remove("topics")
            .map_or(Ok(Vec::new()), Vec::<String>::deserialize)
            .map(|topics| {
                topics
                    .iter()
                    .map(|topic| Arc::new(Topic::new(topic)))
                    .collect()
            })
            .map_err(Error::custom)?;

        Ok(Self {
            term: Deserialize::deserialize(term).map_err(Error::custom)?,
            definition: Deserialize::deserialize(definition).map_err(Error::custom)?,
            topics,
        })
    }
}

impl<'de> Deserialize<'de> for CardSide {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        match value {
            toml::Value::String(text) => Ok(Self {
                text,
                format: CardFormat::Markdown,
            }),
            toml::Value::Table(mut table) => {
                let format = table
                    .remove("format")
                    .map(|format| CardFormat::deserialize(format).map_err(Error::custom))
                    .transpose()?
                    .unwrap_or(CardFormat::default());

                let text = match table.remove("text") {
                    Some(text) => Deserialize::deserialize(text).map_err(Error::custom),
                    None => return Err(Error::custom("missing text")),
                }?;

                Ok(Self { text, format })
            }
            _ => Err(Error::custom("Invalid CardSide")),
        }
    }
}
