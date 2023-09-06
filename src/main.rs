use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router, ServiceExt,
};
use collection::DocumentCollection;
use html_builder::prelude::*;
use itertools::Itertools;
use serve::NodeExt;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use tower_http::normalize_path::NormalizePath;
use tower_service::Service;

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

// #[shuttle_runtime::main]
// async fn main() -> shuttle_axum::ShuttleAxum {
#[tokio::main]
async fn main() {
    let collection = DocumentCollection::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data")).unwrap();
    let cards = Vec::<Card>::try_from(collection)
        .unwrap()
        .into_iter()
        .map(Arc::new)
        .collect_vec();

    let topics = Topics::new(cards.iter().cloned());

    let app = Router::new()
        .route("/", get(serve::index))
        .route("/view", get(serve::view))
        .route("/study", get(serve::study::get).post(serve::study::post))
        .fallback(|req: Request<Body>| async move {
            (
                StatusCode::NOT_FOUND,
                html::main()
                    .class("grid place-items-center grow")
                    .child(h1().text(format!("Page {} not found", req.uri())))
                    .document(),
            )
        })
        .with_state(Arc::new(topics));

    let app = NormalizePath::trim_trailing_slash(app);

    let mut hasher = Sha256::new();
    hasher.update("test123");
    let digest = hasher.finalize();

    let app = serve::auth::Auth::new(app, digest);

    // Ok(app.into())

    let addr = "127.0.0.1:8000".parse().unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
