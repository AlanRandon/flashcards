#![warn(clippy::pedantic)]

use collection::DocumentCollection;
use itertools::Itertools;
use router::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

mod collection;
mod render;
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

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[Arc<Card>]> {
        self.0.get(name).map(Vec::as_slice)
    }
}

fn create_topics(path: impl AsRef<Path>) -> Topics {
    let collection = DocumentCollection::new(path).unwrap();
    let cards = Vec::<Card>::try_from(collection)
        .unwrap()
        .into_iter()
        .map(Arc::new)
        .collect_vec();

    Topics::new(cards.iter().cloned())
}

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn main(
    #[shuttle_secrets::Secrets] secret_store: shuttle_secrets::SecretStore,
) -> Result<impl shuttle_runtime::Service, shuttle_runtime::Error> {
    let topics = create_topics("data");

    // hyper

    let mut hasher = Sha256::new();
    hasher.update(secret_store.get("PASSWORD").unwrap());
    let digest = hasher.finalize();

    Ok(serve::App { digest, topics })
}
