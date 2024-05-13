#![warn(clippy::pedantic)]

use collection::DocumentCollection;
use itertools::Itertools;
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
pub struct Topics {
    topics: HashMap<Arc<str>, Vec<Arc<RenderedCard>>>,
}

impl Topics {
    pub fn new(cards: &[Arc<RenderedCard>]) -> Self {
        let mut topics = HashMap::<_, Vec<_>>::new();
        for card in cards {
            for topic in &card.card.topics {
                topics
                    .entry(Arc::clone(topic))
                    .or_default()
                    .push(Arc::clone(card));
            }
        }
        Self { topics }
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[Arc<RenderedCard>]> {
        self.topics.get(name).map(Vec::as_slice)
    }
}

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn main(
    #[shuttle_runtime::Secrets] secret_store: shuttle_runtime::SecretStore,
) -> Result<impl shuttle_runtime::Service, shuttle_runtime::Error> {
    let collection = DocumentCollection::new("data").unwrap();
    let cards = Vec::<Card>::try_from(collection).unwrap();

    let cards = std::thread::spawn(|| {
        cards
            .into_iter()
            .map(RenderedCard::try_from)
            .map_ok(Arc::new)
            .collect::<Result<Vec<_>, _>>()
    })
    .join()
    .expect("Rendering cards panicked")
    .expect("Rendering cards errored");

    let topics = Topics::new(&cards);

    let password = secret_store.get("PASSWORD").unwrap();

    Ok(serve::App { password, topics })
}
