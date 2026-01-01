#![warn(clippy::pedantic)]
#![feature(int_roundings)]

use askama::Template;
use poem::EndpointExt;
use poem::http::{HeaderMap, HeaderValue, StatusCode};
use poem::middleware::AddData;
use poem::web::{Data, Path, Query};
use serde::Deserialize;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
use std::sync::Arc;

const STYLE: &'static str = include_str!(concat!(env!("OUT_DIR"), "/style.css"));
const SCRIPT: &'static str = include_str!(concat!(env!("OUT_DIR"), "/main.js"));

fn internal_error() -> poem::Response {
    poem::Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(())
}

trait TemplateResponse: Template {
    const STATUS: StatusCode = StatusCode::OK;

    fn into_response(&self) -> poem::Response {
        let Ok(body) = self.render() else {
            return internal_error();
        };

        poem::Response::builder()
            .content_type("text/html")
            .status(Self::STATUS)
            .body(body)
    }
}

#[derive(Debug, askama::Template)]
#[template(path = "index.html")]
struct IndexPage {
    topics: Vec<NamedHash>,
    query: String,
}

impl TemplateResponse for IndexPage {}

#[derive(Debug, askama::Template)]
#[template(path = "index.html", block = "search_results")]
struct IndexPageSearch {
    topics: Vec<NamedHash>,
}

impl TemplateResponse for IndexPageSearch {}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[poem::handler]
async fn index(
    pool: Data<&Arc<SqlitePool>>,
    query: Query<SearchQuery>,
    headers: &HeaderMap,
) -> poem::Response {
    let pattern = match &query.q {
        Some(query) => format!("%{query}%"),
        None => "%".to_string(),
    };

    let Ok(topics) = sqlx::query_as!(
        NamedHash,
        r#"WITH RECURSIVE ancestors AS (
            SELECT parent, name, name AS full_name, hash AS start_hash FROM topic
            UNION ALL
            SELECT topic.parent, ancestors.name, topic.name || '/' || ancestors.full_name AS full_name, ancestors.start_hash
            FROM topic JOIN ancestors ON topic.hash = ancestors.parent
        )
        SELECT start_hash AS "hash!", ancestors.full_name AS name
        FROM ancestors
        WHERE ancestors.name LIKE ? AND ancestors.parent IS NULL
        GROUP BY start_hash"#,
        pattern
    )
    .fetch_all(pool.as_ref())
    .await
    else {
        return internal_error();
    };

    if headers.get("hx-partial") == Some(&HeaderValue::from_static("true")) {
        IndexPageSearch { topics }.into_response()
    } else {
        IndexPage {
            topics,
            query: query.q.to_owned().unwrap_or_else(|| String::new()),
        }
        .into_response()
    }
}

#[derive(Debug)]
struct Card {
    term: String,
    definition: String,
}

#[derive(Debug)]
struct NamedHash {
    hash: i64,
    name: String,
}

const PAGE_SIZE: i64 = 50;

#[derive(Debug, askama::Template)]
#[template(path = "view.html")]
struct View {
    children: Vec<NamedHash>,
    ancestors: Vec<NamedHash>,
    cards: Vec<Card>,
    hash: i64,
    total_cards: i64,
    total_pages: i64,
    current_page: i64,
}

impl TemplateResponse for View {}

async fn topic_ancestors(pool: &SqlitePool, topic: i64) -> sqlx::Result<Vec<NamedHash>> {
    sqlx::query_as!(
        NamedHash,
        "WITH RECURSIVE ancestors AS (
            SELECT hash, parent, name, 0 AS depth
            FROM topic WHERE hash = ?
            UNION ALL
            SELECT topic.hash, topic.parent, topic.name, ancestors.depth + 1
            FROM topic JOIN ancestors ON topic.hash = ancestors.parent
        )
        SELECT hash, ancestors.name AS name FROM ancestors
        ORDER BY depth DESC",
        topic
    )
    .fetch_all(pool)
    .await
}

#[derive(Deserialize)]
struct ViewQuery {
    page: Option<i64>,
}

#[poem::handler]
async fn view(
    pool: Data<&Arc<SqlitePool>>,
    Path(hash): Path<i64>,
    query: Query<ViewQuery>,
) -> poem::Response {
    let Ok(ancestors) = topic_ancestors(&pool, hash).await else {
        return internal_error();
    };

    let Ok(children) = sqlx::query_as!(
        NamedHash,
        r#"SELECT hash AS "hash!", name FROM topic WHERE parent = ?"#,
        hash
    )
    .fetch_all(pool.as_ref())
    .await
    else {
        return internal_error();
    };

    let Ok(total_cards) = sqlx::query!(
        "SELECT COUNT(card) AS total_cards FROM card_topic WHERE card_topic.topic = ?",
        hash,
    )
    .fetch_one(pool.as_ref())
    .await
    else {
        return internal_error();
    };

    let total_cards = total_cards.total_cards;
    let current_page = query.page.unwrap_or(0);
    let total_pages = total_cards.div_ceil(PAGE_SIZE);
    let offset = current_page * PAGE_SIZE;

    let Ok(cards) = sqlx::query_as!(
        Card,
        "SELECT term.html AS term, definition.html AS definition FROM topic
         INNER JOIN card_topic ON card_topic.topic = topic.hash
         INNER JOIN card ON card_topic.card = card.hash
         INNER JOIN rendered AS term ON card.term = term.hash
         INNER JOIN rendered AS definition ON card.definition = definition.hash
         WHERE topic.hash = ?
         GROUP BY card.hash
         ORDER BY card.hash
         LIMIT ?
         OFFSET ?",
        hash,
        PAGE_SIZE,
        offset,
    )
    .fetch_all(pool.as_ref())
    .await
    else {
        return internal_error();
    };

    View {
        children,
        ancestors,
        cards,
        hash,
        total_cards,
        total_pages,
        current_page,
    }
    .into_response()
}

#[derive(Debug, askama::Template)]
#[template(path = "study.html")]
struct Study {
    card: Card,
    total_cards: i64,
    index: i64,
    topics: Vec<NamedHash>,
    topic_ancestors: Vec<NamedHash>,
    topic_hash: i64,
}

impl TemplateResponse for Study {}

#[derive(Deserialize)]
struct StudyQuery {
    index: Option<i64>,
}

#[poem::handler]
async fn study(
    pool: Data<&Arc<SqlitePool>>,
    Path(topic_hash): Path<i64>,
    query: Query<StudyQuery>,
) -> poem::Response {
    let Ok(topic_ancestors) = topic_ancestors(&pool, topic_hash).await else {
        return internal_error();
    };

    let Some(idx) = query.index else {
        return poem::Response::builder()
            .status(StatusCode::TEMPORARY_REDIRECT)
            .header("location", format!("/study/{}?index={}", topic_hash, 0))
            .body(());
    };

    let Ok(total_cards) = sqlx::query!(
        "SELECT COUNT(card) AS total_cards FROM card_topic WHERE card_topic.topic = ?",
        topic_hash
    )
    .fetch_one(pool.as_ref())
    .await
    else {
        return internal_error();
    };

    let Ok(card) = sqlx::query!(
        "SELECT term.html AS term, definition.html AS definition, card.hash FROM card
        INNER JOIN rendered AS term ON card.term = term.hash
        INNER JOIN rendered AS definition ON card.definition = definition.hash
        INNER JOIN card_topic ON card_topic.card = card.hash
        WHERE card_topic.topic = ?
        GROUP BY card.hash
        LIMIT 1
        OFFSET ?",
        topic_hash,
        idx
    )
    .fetch_one(pool.as_ref())
    .await
    else {
        return internal_error();
    };

    let Ok(topics) = sqlx::query_as!(
        NamedHash,
        r#"WITH RECURSIVE ancestors AS (
            SELECT parent, name, name AS full_name, hash AS start_hash FROM topic
            UNION ALL
            SELECT topic.parent, ancestors.name, topic.name || '/' || ancestors.full_name AS full_name, ancestors.start_hash
            FROM topic JOIN ancestors ON topic.hash = ancestors.parent
        )
        SELECT start_hash AS "hash!", ancestors.full_name AS "name!: String"
        FROM ancestors
        INNER JOIN card_topic ON start_hash = card_topic.topic
        WHERE ancestors.parent IS NULL AND card_topic.card = ?"#,
        card.hash,
    )
    .fetch_all(pool.as_ref())
    .await
    else {
        return internal_error();
    };

    Study {
        card: Card {
            term: card.term,
            definition: card.definition,
        },
        total_cards: total_cards.total_cards,
        index: idx,
        topic_ancestors,
        topic_hash,
        topics,
    }
    .into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = Arc::new(
        SqlitePool::connect_with(SqliteConnectOptions::from_str(&database_url)?.read_only(true))
            .await?,
    );

    let app = poem::Route::new()
        .at("/", poem::get(index))
        .at("/view/:hash", poem::get(view))
        .at("/study/:hash", poem::get(study))
        .with(AddData::new(pool));

    let listener = poem::listener::TcpListener::bind("127.0.0.1:8000");
    println!("Listening on http://localhost:8000");
    Ok(poem::Server::new(listener).run(app).await?)
}
