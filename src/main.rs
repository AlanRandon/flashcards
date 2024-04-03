#![warn(clippy::pedantic)]

use collection::DocumentCollection;
use render::RenderedCard;
use std::collections::HashMap;
use std::sync::Arc;

mod collection;
mod render;
mod serve;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardFormat {
    #[default]
    Markdown,
    Tex,
}

#[derive(Debug)]
pub struct CardSide {
    text: String,
    format: CardFormat,
}

#[derive(Debug)]
pub struct Card {
    term: CardSide,
    definition: CardSide,
    topics: Vec<Arc<str>>,
}

#[derive(Debug)]
pub struct Topics(HashMap<Arc<str>, Vec<Arc<RenderedCard>>>);

impl Topics {
    pub fn new(cards: impl Iterator<Item = Card>) -> Result<Self, render::Error> {
        let mut topics = HashMap::<_, Vec<_>>::new();
        for card in cards {
            let card = Arc::new(RenderedCard::try_from(card)?);
            for topic in &card.card.topics {
                topics
                    .entry(Arc::clone(topic))
                    .or_default()
                    .push(Arc::clone(&card));
            }
        }
        Ok(Self(topics))
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[Arc<RenderedCard>]> {
        self.0.get(name).map(Vec::as_slice)
    }
}

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn main(
    #[shuttle_runtime::Secrets] secret_store: shuttle_runtime::SecretStore,
) -> Result<impl shuttle_runtime::Service, shuttle_runtime::Error> {
    let collection = DocumentCollection::new("data").unwrap();
    let cards = Vec::<Card>::try_from(collection).unwrap();

    // Process flashcards on another thread because tectonic uses reqwest, which wants to handle
    // its own async world
    let topics = std::thread::spawn(|| Topics::new(cards.into_iter()).unwrap())
        .join()
        .unwrap();

    let password = secret_store.get("PASSWORD").unwrap();

    Ok(serve::App { password, topics })
}
