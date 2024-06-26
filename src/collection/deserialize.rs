use super::Document;
use crate::{Card, CardFormat, CardSide, Topic};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use toml::Table;

impl Document {
    pub fn deserialize_with_topics<'de, D>(
        deserializer: D,
        topics: &[Topic],
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
                .map(|topic| Topic::new(topic)),
        );

        let mut cards = table
            .remove("cards")
            .map_or(Ok(Vec::new()), Vec::<Card>::deserialize)
            .map_err(Error::custom)?;

        for card in &mut cards {
            card.topics.extend(topics.iter().cloned());
        }

        Ok(Document(cards))
    }

    pub fn deserialize_toml_with_topics(
        document: &str,
        topics: &[Topic],
    ) -> Result<Self, toml::de::Error> {
        Self::deserialize_with_topics(toml::de::Deserializer::new(document), topics)
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
            .map(|topics| topics.iter().map(|topic| Topic::new(topic)).collect())
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
