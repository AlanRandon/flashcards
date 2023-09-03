use collection::DocumentCollection;
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};

mod collection;
mod serve;

#[derive(Debug)]
pub struct Card {
    term: String,
    definition: String,
    topics: Vec<Arc<str>>,
}

#[derive(Debug)]
pub struct Topics(HashMap<Arc<str>, Vec<Arc<Card>>>);

impl Topics {
    pub fn new(cards: impl Iterator<Item = Arc<Card>>) -> Self {
        let mut topics = HashMap::<_, Vec<_>>::new();
        for card in cards {
            for topic in &card.topics {
                topics
                    .entry(Arc::clone(topic))
                    .or_default()
                    .push(Arc::clone(&card));
            }
        }
        Self(topics)
    }

    pub fn get(&self, name: &str) -> Option<&[Arc<Card>]> {
        self.0.get(name).map(|topic| topic.as_slice())
    }
}

#[tokio::main]
async fn main() {
    let collection = DocumentCollection::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data")).unwrap();
    let cards = Vec::<Card>::try_from(collection)
        .unwrap()
        .into_iter()
        .map(Arc::new)
        .collect_vec();

    let topics = Topics::new(cards.iter().cloned());

    serve::serve(topics).await.unwrap();
}
