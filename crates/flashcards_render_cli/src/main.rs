use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("error loading cards: {0}")]
    Io(#[from] std::io::Error),
    #[error("error deserializing cards at {path}: {err}")]
    Deserialize {
        path: PathBuf,
        #[source]
        err: toml::de::Error,
    },
    #[error("error rendering card at {path}: {err}")]
    Render {
        path: PathBuf,
        #[source]
        err: flashcards_render::render::Error,
    },
    #[error("error accessing database: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid utf8 in path")]
    NonUtf8Path(PathBuf),
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

async fn load(
    path: impl AsRef<Path>,
    progress: &MultiProgress,
) -> Result<Vec<Card<flashcards_render::Source>>, Error> {
    let load_progess = ProgressBar::new_spinner()
        .with_style(ProgressStyle::with_template("{msg} {spinner}").unwrap())
        .with_message(section_title("Loading", SectionTitleState::Processing));
    let load_progress = progress.add(load_progess);

    let cards = flashcards_render::loader::load_dir(path)
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
        .collect::<Result<Vec<_>, _>>()?;

    load_progress.finish();
    progress.remove(&load_progress);
    _ = progress.println(section_title("Loaded", SectionTitleState::Done));

    Ok(cards)
}

struct RenderedCard {
    term: i64,
    definition: i64,
    topics: HashSet<Arc<flashcards_render::Topic>>,
    path: Arc<PathBuf>,
}

async fn render(
    pool: Arc<SqlitePool>,
    cards: Vec<Card<flashcards_render::Source>>,
    progress: &MultiProgress,
) -> Result<Vec<RenderedCard>, Error> {
    let render_progress = ProgressBar::new(
        cards
            .len()
            .try_into()
            .expect("number of cards to fit in u64"),
    )
    .with_style(bar_style("Rendering"));
    let render_progress = progress.add(render_progress);

    let mut render_jobs = tokio::task::JoinSet::<Result<_, Error>>::new();
    for card in cards {
        let render_progress = render_progress.clone();
        let pool = Arc::clone(&pool);
        render_jobs.spawn(async move {
            let term =
                render_source_cached(card.card.term, card.path.as_path(), &pool, &render_progress)
                    .await?;

            let definition = render_source_cached(
                card.card.definition,
                card.path.as_path(),
                &pool,
                &render_progress,
            )
            .await?;

            render_progress.inc(1);

            Ok(RenderedCard {
                path: card.path,
                topics: card.card.topics,
                term,
                definition,
            })
        });
    }

    let mut cards = Vec::new();
    while let Some(result) = render_jobs.join_next().await {
        let card = result.expect("render job not to panic or be cancelled")?;
        cards.push(card);
    }

    progress.remove(&render_progress);
    _ = progress.println(section_title("Rendered", SectionTitleState::Done));

    Ok(cards)
}

async fn index(
    pool: &SqlitePool,
    cards: &[RenderedCard],
    progress: &MultiProgress,
) -> Result<(), Error> {
    let index_progress = ProgressBar::new(
        cards
            .len()
            .try_into()
            .expect("number of cards to fit in u64"),
    )
    .with_style(bar_style("Indexing"));
    let index_progress = progress.add(index_progress);

    sqlx::query!("DELETE FROM card_topic")
        .execute(&*pool)
        .await?;

    sqlx::query!("DELETE FROM topic").execute(&*pool).await?;

    let compiled_time = chrono::Utc::now();

    for card in cards {
        let mut hasher = std::hash::DefaultHasher::new();
        card.term.hash(&mut hasher);
        card.definition.hash(&mut hasher);
        for topic in card.topics.iter() {
            topic.hash(&mut hasher);
        }
        let hash = i64::from_ne_bytes(hasher.finish().to_ne_bytes());

        let path = card
            .path
            .to_str()
            .ok_or_else(|| Error::NonUtf8Path(card.path.to_path_buf()))?;

        sqlx::query!(
            "INSERT OR REPLACE INTO card (hash, term, definition, source_path, compiled_at) VALUES (?, ?, ?, ?, ?)",
            hash,
            card.term,
            card.definition,
            path,
            compiled_time,
        )
        .execute(&*pool)
        .await?;

        insert_card_topics(&card.topics, hash, &pool).await?;

        index_progress.inc(1);
    }

    sqlx::query!("DELETE FROM card WHERE compiled_at != ?", compiled_time)
        .execute(&*pool)
        .await?;

    progress.remove(&index_progress);
    _ = progress.println(section_title("Indexed", SectionTitleState::Done));

    Ok(())
}

async fn run(pool: Arc<SqlitePool>, path: impl AsRef<Path>) -> Result<(), Error> {
    let progress = MultiProgress::new();

    let cards = load(&path, &progress).await?;
    let cards = render(Arc::clone(&pool), cards, &progress).await?;
    index(&pool, &cards, &progress).await?;

    Ok(())
}

fn report_error(err: impl Display) {
    eprintln!("\u{1b}[31;1merror\u{1b}[0m: {err}")
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: SqliteConnectOptions,
    #[arg(default_value = "data")]
    input: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let pool = match SqlitePool::connect_with(cli.database_url)
        .await
        .map(Arc::new)
    {
        Ok(pool) => pool,
        Err(err) => {
            report_error(format!("failed to connect to database: {err}",));
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = sqlx::migrate!("../../migrations").run(pool.as_ref()).await {
        report_error(format!("failed run database migrations: {err}"));
        return ExitCode::FAILURE;
    }

    if let Err(err) = run(pool, "data").await {
        report_error(err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
