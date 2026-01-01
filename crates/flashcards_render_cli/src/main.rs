use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::collections::{HashMap, HashSet};
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

#[derive(Debug)]
struct TopicData {
    cards: HashSet<i64>,
    parent: Option<i64>,
    name: Arc<str>,
    full_name: String,
    length: usize,
}

async fn index(
    pool: &Arc<SqlitePool>,
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

    index_progress.set_message(": cards");

    let compiled_time = chrono::Utc::now();

    let mut topic_data = HashMap::new();

    for card in cards.iter() {
        let mut hasher = std::hash::DefaultHasher::new();
        card.term.hash(&mut hasher);
        card.definition.hash(&mut hasher);

        let mut topic_hashes = HashSet::new();
        for topic in card.topics.iter() {
            let mut hasher = std::hash::DefaultHasher::new();
            for segment in topic.0.iter().take(topic.0.len() - 1) {
                segment.hash(&mut hasher);
            }

            let name = topic.0.last().expect("topic to have at least 1 segment");

            let topic_parent_hash = if topic.0.len() > 1 {
                Some(i64::from_ne_bytes(hasher.finish().to_ne_bytes()))
            } else {
                None
            };

            name.hash(&mut hasher);
            let topic_hash = i64::from_ne_bytes(hasher.finish().to_ne_bytes());

            topic_data.entry(topic_hash).or_insert(TopicData {
                cards: HashSet::new(),
                parent: topic_parent_hash,
                name: Arc::clone(name),
                full_name: topic.0.join("/"),
                length: topic.0.len(),
            });

            topic_hashes.insert(topic_hash);
            topic_hash.hash(&mut hasher);
        }

        let hash = i64::from_ne_bytes(hasher.finish().to_ne_bytes());

        for topic in topic_hashes {
            topic_data.entry(topic).and_modify(|data| {
                data.cards.insert(hash);
            });
        }

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
        .persistent(true)
        .execute(pool.as_ref())
        .await?;

        index_progress.inc(1);
    }

    sqlx::query!("DELETE FROM card WHERE compiled_at != ?", compiled_time)
        .execute(pool.as_ref())
        .await?;

    index_progress.set_length(topic_data.len() as u64);
    index_progress.set_position(0);
    index_progress.set_message(": topics");

    let topic_progress = ProgressBar::new(0).with_style(bar_style("Indexing"));
    let topic_progress = progress.add(topic_progress);

    for (topic, data) in topic_data
        .iter()
        .sorted_unstable_by(|a, b| a.1.length.cmp(&b.1.length))
    {
        let name = data.name.as_ref();
        topic_progress.set_length(data.cards.len() as u64);
        topic_progress.set_position(0);
        topic_progress.set_message(format!(": {}", data.full_name));

        sqlx::query!(
            "INSERT OR IGNORE INTO topic (hash, name, parent) VALUES (?, ?, ?)",
            topic,
            name,
            data.parent,
        )
        .persistent(true)
        .execute(pool.as_ref())
        .await?;

        for card in data.cards.iter().copied() {
            let pool = Arc::clone(pool);
            let topic = *topic;

            sqlx::query!(
                "INSERT OR IGNORE INTO card_topic (card, topic) VALUES (?, ?)",
                card,
                topic,
            )
            .persistent(true)
            .execute(pool.as_ref())
            .await?;

            topic_progress.inc(1);
        }

        index_progress.inc(1);
    }

    progress.remove(&topic_progress);
    index_progress.set_message(": cleaning");

    sqlx::query!(
        "DELETE FROM topic
        WHERE topic.hash NOT IN (
            SELECT card_topic.topic FROM card_topic
            INNER JOIN card ON card_topic.card = card.hash
            WHERE card_topic.topic = topic.hash
            AND card.compiled_at = ?
        )",
        compiled_time
    )
    .execute(pool.as_ref())
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
