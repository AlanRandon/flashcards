#![warn(clippy::pedantic)]

use collection::DocumentCollection;
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

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

// #[tokio::main]
// async fn main() {
//     let topics = create_topics(path);

//     let mut hasher = Sha256::new();
//     hasher.update("test123");
//     let digest = hasher.finalize();

//     serve::App {
//         digest,
//         topics,
//     }
//     .run("127.0.0.1:8000".parse().unwrap())
//     .await;
// }

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for serve::App {
    async fn bind(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        self.bind(&addr).await;
        Ok(())
    }
}

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn main(
    #[shuttle_static_folder::StaticFolder(folder = "data")] data: PathBuf,
    #[shuttle_static_folder::StaticFolder(folder = "dist")] dist: PathBuf,
    #[shuttle_secrets::Secrets] secret_store: shuttle_secrets::SecretStore,
) -> Result<serve::App, shuttle_runtime::Error> {
    let topics = create_topics(data);

    unsafe {
        serve::STYLE_CSS = std::fs::read_to_string(dist.join("style.css")).unwrap();
        serve::INIT_JS = std::fs::read_to_string(dist.join("init.js")).unwrap();
    }

    let mut hasher = Sha256::new();
    hasher.update(secret_store.get("PASSWORD").unwrap());
    let digest = hasher.finalize();

    Ok(serve::App { digest, topics })
}
