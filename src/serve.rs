use crate::collection::deserialize::Topic;
use crate::{RenderedCard, Topics};
use askama::Template;
use http::StatusCode;
use http_body_util::BodyExt;
use router::prelude::*;
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::Arc;

pub mod auth;
mod response;
mod search;
mod study;

pub type Request<'req> = router::Request<'req, hyper::body::Bytes, Context<'req>>;
pub type Response = http::Response<http_body_util::Full<bytes::Bytes>>;

pub struct Context<'req> {
    topics: &'req Topics,
    password: Arc<str>,
}

trait RequestExt<'req> {
    fn is_htmx(&self) -> bool;
    fn query<T: Deserialize<'req>>(&self) -> Result<T, serde_urlencoded::de::Error>;
    async fn form<T: Deserialize<'req>>(&self) -> Result<T, serde_urlencoded::de::Error>;
}

impl<'req> RequestExt<'req> for Request<'req> {
    fn is_htmx(&self) -> bool {
        self.request.headers().contains_key("Hx-Request")
    }

    fn query<T: Deserialize<'req>>(&self) -> Result<T, serde_urlencoded::de::Error> {
        let query = self.request.uri().query().unwrap_or("");
        serde_urlencoded::from_str(query)
    }

    async fn form<T: Deserialize<'req>>(&self) -> Result<T, serde_urlencoded::de::Error> {
        let body = serde_urlencoded::from_bytes(self.request.body())?;
        Ok(body)
    }
}

#[derive(Template)]
#[template(
    source = r#"<main class="grid place-items-center grow"><h1>{{ err }}</h1></main>"#,
    ext = "html"
)]
struct Error<'a> {
    err: &'a str,
}

const PAGE_SIZE: usize = 50;

#[derive(Debug, Deserialize)]
pub struct TopicQuery<'r> {
    name: Cow<'r, str>,
    page: Option<usize>,
}

#[derive(Template)]
#[template(path = "view.html")]
struct View<'a> {
    topic: Topic,
    subtopics: Vec<&'a Topic>,
    cards: &'a [Arc<RenderedCard>],
    card_number: usize,
    page: usize,
    max_page: usize,
}

#[get("view")]
fn view(request: &Request<'req>) -> Response {
    let Ok(query) = request.query::<TopicQuery>() else {
        return response::partial_if(
            &Error {
                err: "Invalid Query Parameters",
            },
            StatusCode::BAD_REQUEST,
            request.is_htmx(),
        );
    };

    let topic = Topic::new(&query.name);

    let Some(cards) = request.context.topics.get(&topic) else {
        return response::partial_if(
            &Error {
                err: "Set Not Found",
            },
            StatusCode::NOT_FOUND,
            request.is_htmx(),
        );
    };

    let subtopics = request
        .context
        .topics
        .topics
        .keys()
        .filter(|subtopic| subtopic.0.get(0..topic.0.len()) == Some(&topic.0))
        .filter(|subtopic| subtopic.0.len() == topic.0.len() + 1)
        .map(AsRef::as_ref)
        .collect();

    let card_number = cards.len();

    let max_page = cards.len() / PAGE_SIZE;
    let page = query.page.unwrap_or(0).min(max_page);
    let cards =
        &cards[(page * PAGE_SIZE).min(cards.len())..((page + 1) * PAGE_SIZE).min(cards.len())];

    response::partial_if(
        &View {
            topic,
            subtopics,
            cards,
            card_number,
            page,
            max_page,
        },
        StatusCode::OK,
        request.is_htmx(),
    )
}

router![async Router => view, search::index, search::search, study::study];

pub struct App {
    pub password: String,
    pub topics: Topics,
}

impl App {
    #[allow(clippy::too_many_lines)]
    async fn run(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        use hyper::service::service_fn;
        use hyper_util::rt::{TokioExecutor, TokioIo};
        use hyper_util::server::conn::auto;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind(addr).await?;

        let Self {
            password, topics, ..
        } = self;
        let topics = Arc::new(topics);
        let password = Arc::<str>::from(password.into_boxed_str());
        let dist_files =
            hyper_staticfile::Static::new(concat!(env!("CARGO_MANIFEST_DIR"), "/dist"));

        let listen = tokio::task::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };

                let io = TokioIo::new(stream);

                tokio::task::spawn({
                    let topics = Arc::clone(&topics);
                    let password = Arc::clone(&password);
                    let dist_files = dist_files.clone();
                    async move {
                        let service =
                            service_fn(move |request: http::Request<hyper::body::Incoming>| {
                                let topics = Arc::clone(&topics);
                                let password = Arc::clone(&password);
                                let dist_files = dist_files.clone();
                                async move {
                                    let (parts, body) = request.into_parts();
                                    let request = http::Request::from_parts(
                                        parts,
                                        body.collect().await?.to_bytes(),
                                    );

                                    let topics = topics.as_ref();

                                    let context = Context { topics, password };

                                    let request =
                                        Request::from_http_with_context(&request, &context);

                                    Ok::<_, hyper::Error>(
                                        auth::login(&request, move |request| async move {
                                            if let Some(response) = Router::route(request).await {
                                                return response;
                                            }

                                            let response = match dist_files
                                                .serve({
                                                    let (parts, _) =
                                                        request.request.clone().into_parts();
                                                    http::Request::from_parts(parts, ())
                                                })
                                                .await
                                            {
                                                Ok(response) => {
                                                    let (parts, body) = response.into_parts();
                                                    let body = body.collect().await;
                                                    body.map(|body| {
                                                        http::Response::from_parts(
                                                            parts,
                                                            body.to_bytes().into(),
                                                        )
                                                    })
                                                }
                                                Err(err) => Err(err),
                                            };

                                            match response {
                                                Ok(response) => response,
                                                Err(err)
                                                    if matches!(
                                                        err.kind(),
                                                        std::io::ErrorKind::NotFound
                                                    ) =>
                                                {
                                                    response::partial_if(
                                                        &Error {
                                                            err: &format!(
                                                                "Not found: {}",
                                                                request.request.uri()
                                                            ),
                                                        },
                                                        StatusCode::NOT_FOUND,
                                                        request.is_htmx(),
                                                    )
                                                }
                                                Err(_) => response::partial_if(
                                                    &Error {
                                                        err: "Unknown Error",
                                                    },
                                                    StatusCode::INTERNAL_SERVER_ERROR,
                                                    request.is_htmx(),
                                                ),
                                            }
                                        })
                                        .await,
                                    )
                                }
                            });

                        if let Err(err) = auto::Builder::new(TokioExecutor::new())
                            .serve_connection(io, service)
                            .await
                        {
                            println!("Error serving connection: {err:?}");
                        }
                    }
                });
            }
        });

        println!("Listening on {addr}");

        listen.await.unwrap()
    }
}

impl shuttle_runtime::Service for App {
    fn bind<'async_trait>(
        self,
        addr: std::net::SocketAddr,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<(), shuttle_runtime::Error>>
                + Send
                + 'async_trait,
        >,
    >
    where
        Self: 'async_trait,
    {
        Box::pin(self.run(addr))
    }
}
