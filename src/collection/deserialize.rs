use super::Document;
use crate::Card;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::sync::Arc;
use toml::Table;

impl Document {
    pub fn deserialize_with_topics<'de, D>(
        deserializer: D,
        topics: &[Arc<str>],
    ) -> Result<Document, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut table = Table::deserialize(deserializer)?;
        let mut topics = topics.to_owned();

        topics.extend(
            table
                .remove("topics")
                .map_or(Ok(Vec::new()), Vec::<Arc<str>>::deserialize)
                .map_err(Error::custom)?,
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
        topics: &[Arc<str>],
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
            .map_or(Ok(Vec::new()), Vec::<Arc<str>>::deserialize)
            .map_err(Error::custom)?;

        Ok(Self {
            term: String::deserialize(term).map_err(Error::custom)?,
            definition: String::deserialize(definition).map_err(Error::custom)?,
            topics,
        })
    }
}
