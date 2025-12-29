use crate::{Card, Format, Source, Topic};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::convert::Infallible;
use std::str::FromStr;
use std::sync::Arc;

impl FromStr for Topic {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let segments = s
            .split('/')
            .map(|segment| segment.to_string().into_boxed_str().into())
            .collect::<Vec<_>>();

        Ok(Self(segments))
    }
}

impl Topic {
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

pub fn parse(source: &str) -> Result<Vec<Card<Source>>, toml::de::Error> {
    let mut table = source.parse::<toml::Table>()?;

    let topics = parse_topics(&mut table)?;

    let mut cards = match table.remove("cards") {
        Some(toml::Value::Array(cards)) => cards
            .into_iter()
            .map(parse_card)
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
        Some(_) => return Err(Error::custom("missing cards")),
    };

    for card in cards.iter_mut() {
        card.topics.extend(topics.iter().cloned());
    }

    Ok(cards)
}

fn parse_card(card: toml::Value) -> Result<Card<Source>, toml::de::Error> {
    let toml::Value::Table(mut card) = card else {
        return Err(Error::custom("card must be a table"));
    };

    let term = card
        .remove("term")
        .map(Source::deserialize)
        .unwrap_or_else(|| Err(Error::custom("card must have a term")))?;

    let definition = card
        .remove("definition")
        .map(Source::deserialize)
        .unwrap_or_else(|| Err(Error::custom("card must have a definition")))?;

    let topics = parse_topics(&mut card)?;

    Ok(Card {
        term,
        definition,
        topics,
    })
}

fn parse_topics(table: &mut toml::Table) -> Result<HashSet<Arc<Topic>>, toml::de::Error> {
    let mut topics = HashSet::new();

    let Some(topic_strs) = table.remove("topics") else {
        return Ok(topics);
    };

    let toml::Value::Array(topic_strs) = topic_strs else {
        return Err(Error::custom("topic array must be an array"));
    };

    for topic_str in topic_strs {
        let toml::Value::String(topic_str) = topic_str else {
            return Err(Error::custom("topic array must contain only strings"));
        };

        topics.extend(
            topic_str
                .parse::<Topic>()
                .unwrap_or_else(|val| match val {})
                .ancestors()
                .map(Arc::new),
        );
    }

    Ok(topics)
}

impl<'de> Deserialize<'de> for Source {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        match value {
            toml::Value::String(source) => Ok(Self {
                source,
                format: Format::Markdown,
            }),
            toml::Value::Table(mut table) => {
                let format = table
                    .remove("format")
                    .map(|format| Format::deserialize(format).map_err(Error::custom))
                    .transpose()?
                    .unwrap_or(Format::default());

                let source = match table.remove("text") {
                    Some(source) => Deserialize::deserialize(source).map_err(Error::custom),
                    None => return Err(Error::custom("missing text")),
                }?;

                Ok(Self { source, format })
            }
            _ => Err(Error::custom("invalid side of card")),
        }
    }
}
