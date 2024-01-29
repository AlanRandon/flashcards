// TODO: files
// TODO: auth

use crate::render::filters;
use crate::{Card, Topics};
use askama::Template;
use http::StatusCode;
use http_body_util::BodyExt;
use router::prelude::*;
use serde::Deserialize;
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
    key: Arc<cookie::Key>,
}

trait RequestExt<'req> {
    fn is_htmx(&self) -> bool;
    fn query<T: Deserialize<'req>>(&self) -> Result<T, serde_querystring::Error>;
    async fn form<T: Deserialize<'req>>(&self) -> Result<T, serde_querystring::Error>;
}

impl<'req> RequestExt<'req> for Request<'req> {
    fn is_htmx(&self) -> bool {
        self.request.headers().contains_key("Hx-Request")
    }

    fn query<T: Deserialize<'req>>(&self) -> Result<T, serde_querystring::Error> {
        serde_querystring::from_str(
            self.request.uri().query().unwrap_or(""),
            serde_querystring::ParseMode::UrlEncoded,
        )
    }

    async fn form<T: Deserialize<'req>>(&self) -> Result<T, serde_querystring::Error> {
        let body = serde_querystring::from_bytes::<T>(
            self.request.body(),
            serde_querystring::ParseMode::UrlEncoded,
        )?;

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

#[derive(Debug, Deserialize)]
pub struct TopicQuery<'a> {
    name: &'a str,
}

#[derive(Template)]
#[template(path = "view.html")]
struct View<'a> {
    cards: Vec<&'a Card>,
    name: &'a str,
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

    let Some(cards) = request.context.topics.get(query.name) else {
        return response::partial_if(
            &Error {
                err: "Set Not Found",
            },
            StatusCode::NOT_FOUND,
            request.is_htmx(),
        );
    };

    response::partial_if(
        &View {
            name: query.name,
            cards: cards.iter().map(AsRef::as_ref).collect(),
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
    async fn run(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        use hyper::service::service_fn;
        use hyper_util::rt::{TokioExecutor, TokioIo};
        use hyper_util::server::conn::auto;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind(addr).await?;

        let Self { password, topics } = self;
        let topics = Arc::new(topics);
        let key = Arc::new(cookie::Key::generate());
        let password = Arc::<str>::from(password.into_boxed_str());

        let listen = tokio::task::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };

                let io = TokioIo::new(stream);

                tokio::task::spawn({
                    let topics = Arc::clone(&topics);
                    let key = Arc::clone(&key);
                    let password = Arc::clone(&password);
                    async move {
                        if let Err(err) = auto::Builder::new(TokioExecutor::new())
                            .serve_connection(
                                io,
                                service_fn(move |request: http::Request<hyper::body::Incoming>| {
                                    let topics = Arc::clone(&topics);
                                    let key = Arc::clone(&key);
                                    let password = Arc::clone(&password);
                                    async move {
                                        let (parts, body) = request.into_parts();
                                        let request = http::Request::from_parts(
                                            parts,
                                            body.collect().await?.to_bytes(),
                                        );

                                        let topics = topics.as_ref();

                                        let context = Context {
                                            topics,
                                            password,
                                            key,
                                        };

                                        let request =
                                            Request::from_http_with_context(&request, &context);

                                        Ok::<_, hyper::Error>(
                                            auth::login(&request, move |request| async move {
                                                Router::route(request).await.unwrap_or_else(|| {
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
                                                })
                                            })
                                            .await,
                                        )
                                    }
                                }),
                            )
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

// pub fn app(digest: auth::Digest, topics: Topics) -> rocket::Rocket<rocket::Build> {
//     const STATIC_FILES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/dist/static");
//     let static_files = rocket::fs::FileServer::from(STATIC_FILES);

//     rocket::build()
//         .mount(
//             "/",
//             routes![
//                 view,
//                 search::index,
//                 search::search,
//                 study::study,
//                 auth::login,
//             ],
//         )
//         .mount("/static", static_files)
//         .register("/", catchers![auth::catch_unauthorized, catcher])
//         .manage(topics)
//         .manage(digest)
// }
