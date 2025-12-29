use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("error loading cards")]
    Io(#[from] std::io::Error),
    #[error("error deserializing cards")]
    Deserialize {
        path: PathBuf,
        #[source]
        err: toml::de::Error,
    },
    #[error("error rendering card at {path}")]
    Render {
        path: PathBuf,
        #[source]
        err: flashcards_render::render::Error,
    },
    #[error("error accessing database: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug)]
struct Card<T> {
    card: flashcards_render::Card<T>,
    path: Arc<PathBuf>,
}

enum SectionTitleState {
    Processing,
    Done,
}

fn section_title(name: &str, done: SectionTitleState) -> String {
    format!(
        "\u{1b}[{};1m{name:>12}\u{1b}[0m",
        match done {
            SectionTitleState::Processing => "36",
            SectionTitleState::Done => "32",
        }
    )
}

fn bar_style(name: &str) -> ProgressStyle {
    ProgressStyle::with_template(&format!(
        "{}{}",
        section_title(name, SectionTitleState::Processing),
        " [{bar}] {pos}/{len}{msg}",
    ))
    .unwrap()
    .progress_chars("=> ")
}

async fn render_source_cached(
    source: flashcards_render::Source,
    path: impl AsRef<Path>,
    pool: &SqlitePool,
    progress: &ProgressBar,
) -> Result<i64, Error> {
    let mut hasher = std::hash::DefaultHasher::new();
    source.hash(&mut hasher);
    let hash = i64::from_ne_bytes(hasher.finish().to_ne_bytes());

    if sqlx::query!("SELECT hash FROM rendered WHERE hash = (?)", hash)
        .fetch_optional(pool)
        .await?
        .is_some()
    {
        return Ok(hash);
    };

    let path = path.as_ref().to_path_buf();
    progress.set_message(format!(": {} at {}", source.format, path.to_string_lossy()));

    let rendered: flashcards_render::Rendered = tokio_rayon::spawn(move || source.try_into())
        .await
        .map_err(|err| Error::Render { path, err })?;

    let format = rendered.source.format.to_string();

    sqlx::query!(
        "INSERT OR IGNORE INTO rendered (hash, source, html, render_format) VALUES (?, ?, ?, ?)",
        hash,
        rendered.source.source,
        rendered.html,
        format,
    )
    .execute(pool)
    .await?;

    Ok(hash)
}

async fn insert_card_topics(
    topics: &HashSet<Arc<flashcards_render::Topic>>,
    card_hash: i64,
    pool: &SqlitePool,
) -> sqlx::Result<()> {
    let mut txn = pool.begin().await?;

    for flashcards_render::Topic(segments) in topics.into_iter().map(AsRef::as_ref) {
        assert!(segments.len() >= 1);

        let mut parent = None;
        let mut hasher = std::hash::DefaultHasher::new();
        for name in segments.into_iter().map(AsRef::as_ref) {
            name.hash(&mut hasher);
            let hash = i64::from_ne_bytes(hasher.finish().to_ne_bytes());

            sqlx::query!(
                "INSERT OR IGNORE INTO topic (hash, name, parent) VALUES (?, ?, ?)",
                hash,
                name,
                parent,
            )
            .execute(&mut *txn)
            .await?;

            sqlx::query!(
                "INSERT OR IGNORE INTO card_topic (card, topic) VALUES (?, ?)",
                card_hash,
                hash,
            )
            .execute(&mut *txn)
            .await?;

            parent = Some(hash);
        }
    }

    txn.commit().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let progress = MultiProgress::new();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| todo!());
    let pool = Arc::new(
        SqlitePool::connect_with(
            SqliteConnectOptions::from_str(&database_url).unwrap_or_else(|_| todo!()),
        )
        .await
        .unwrap_or_else(|_| todo!()),
    );

    sqlx::migrate!("../../migrations")
        .run(pool.as_ref())
        .await
        .unwrap_or_else(|err| todo!("{}", err));

    let load_progess = ProgressBar::new_spinner()
        .with_style(ProgressStyle::with_template("{msg} {spinner}").unwrap())
        .with_message(section_title("Loading", SectionTitleState::Processing));
    let load_progress = progress.add(load_progess);

    let cards = flashcards_render::loader::load_dir("data")
        .map(|result| -> Result<_, Error> {
            load_progress.inc(1);

            let file_contents = result.map_err(Error::Io)?;
            let path = file_contents.path.clone();
            let cards = file_contents
                .into_cards()
                .map_err(|err| Error::Deserialize {
                    path: path.clone(),
                    err,
                })?;

            let path = Arc::new(path);
            Ok(cards.into_iter().map(move |card| Card {
                card,
                path: Arc::clone(&path),
            }))
        })
        .flatten_ok()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| todo!());

    load_progress.finish();
    progress.remove(&load_progress);
    _ = progress.println(section_title("Loaded", SectionTitleState::Done));

    let render_progress = ProgressBar::new(
        cards
            .len()
            .try_into()
            .expect("number of cards to fit in u64"),
    )
    .with_style(bar_style("Rendering"));
    let render_progress = progress.add(render_progress);

    let mut render_jobs = tokio::task::JoinSet::new();
    for card in cards {
        let render_progress = render_progress.clone();
        let pool = Arc::clone(&pool);
        render_jobs.spawn(async move {
            let term =
                render_source_cached(card.card.term, card.path.as_path(), &pool, &render_progress)
                    .await
                    .unwrap_or_else(|err| todo!("{err:?}"));

            let definition = render_source_cached(
                card.card.definition,
                card.path.as_path(),
                &pool,
                &render_progress,
            )
            .await
            .unwrap_or_else(|err| todo!("{err:?}"));

            render_progress.inc(1);

            (card.path, card.card.topics, term, definition)
        });
    }

    let cards = render_jobs.join_all().await;

    progress.remove(&render_progress);
    _ = progress.println(section_title("Rendered", SectionTitleState::Done));

    let indexing_progress = ProgressBar::new(
        cards
            .len()
            .try_into()
            .expect("number of cards to fit in u64"),
    )
    .with_style(bar_style("Indexing"));
    let indexing_progress = progress.add(indexing_progress);

    for (path, topics, term, definition) in cards {
        let mut hasher = std::hash::DefaultHasher::new();
        term.hash(&mut hasher);
        definition.hash(&mut hasher);
        for topic in topics.iter() {
            topic.hash(&mut hasher);
        }
        let hash = i64::from_ne_bytes(hasher.finish().to_ne_bytes());

        let path = path
            .to_str()
            .unwrap_or_else(|| todo!("path is invalid unicode"));

        sqlx::query!(
            "INSERT OR REPLACE INTO card (hash, term, definition, source_path) VALUES (?, ?, ?, ?)",
            hash,
            term,
            definition,
            path,
        )
        .execute(pool.as_ref())
        .await
        .unwrap_or_else(|err| todo!("{err:?}"));

        insert_card_topics(&topics, hash, &pool)
            .await
            .unwrap_or_else(|err| todo!("{err:?}"));

        indexing_progress.inc(1);
    }

    progress.remove(&render_progress);
    _ = progress.println(section_title("Indexed", SectionTitleState::Done));

    ExitCode::SUCCESS
}
