#![warn(clippy::pedantic)]

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
use std::sync::Arc;

// mod serve;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = Arc::new(
        SqlitePool::connect_with(SqliteConnectOptions::from_str(&database_url)?.read_only(true))
            .await
            .unwrap_or_else(|_| todo!()),
    );

    let password = std::env::var("PASSWORD")?;

    let cards = sqlx::query!(
        "SELECT term.html AS term, definition.html AS definition, card.hash, COUNT(topic) AS topics FROM card
        INNER JOIN rendered AS term ON card.term = term.hash
        INNER JOIN rendered AS definition ON card.definition = definition.hash
        INNER JOIN card_topic ON card_topic.card = card.hash
        INNER JOIN topic ON card_topic.topic = topic.hash
        GROUP BY card.hash
        LIMIT 3"
    )
    .fetch_all(&*pool)
    .await?;

    dbg!(cards);

    Ok(())

    // serve::App { password, topics }
    //     .run(std::net::SocketAddr::from_str("127.0.0.1:8000")?)
    //     .await
}
