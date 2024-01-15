// TODO: files
// TODO: auth
// TODO: study

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

pub type Request<'req> = router::Request<'req, hyper::body::Bytes, &'req Topics>;
pub type Response = http::Response<http_body_util::Full<bytes::Bytes>>;

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
            Error {
                err: "Invalid Query Parameters",
            },
            StatusCode::BAD_REQUEST,
            request.is_htmx(),
        );
    };

    let Some(cards) = request.context.get(query.name) else {
        return response::partial_if(
            Error {
                err: "Set Not Found",
            },
            StatusCode::NOT_FOUND,
            request.is_htmx(),
        );
    };

    response::partial_if(
        View {
            name: query.name,
            cards: cards.iter().map(AsRef::as_ref).collect(),
        },
        StatusCode::OK,
        request.is_htmx(),
    )
}

// #[catch(default)]
// async fn catcher<'a>(
//     status: Status,
//     request: &'a Request<'_>,
// ) -> Either<Response<impl Template + 'a>, Response<impl Template>> {
//     let Outcome::Success(_) = request.guard::<Authed>().await else {
//         return Either::A(auth::catch_unauthorized(request));
//     };

//     let message = if status == Status::NotFound {
//         format!("Page {} not found", request.uri())
//     } else {src
//         "Unknown Error Occurred".to_string()
//     };

//     let htmx = HxRequest::from_request(request).await.unwrap();

//     if htmx.0 {
//         Either::B(Response::partial(Error { err: message }))
//     } else {
//         Either::B(Response::page(Error { err: message }))
//     }
// }

router![async Router => view, search::index, search::search];

pub struct App {
    pub digest: auth::Digest,
    pub topics: Topics,
}

impl App {
    async fn run(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        use hyper::service::service_fn;
        use hyper_util::rt::{TokioExecutor, TokioIo};
        use hyper_util::server::conn::auto;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind(addr).await?;

        let Self { digest, topics } = self;
        let topics = Arc::new(topics);

        let listen = tokio::task::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };

                let io = TokioIo::new(stream);

                tokio::task::spawn({
                    let topics = Arc::clone(&topics);
                    async move {
                        if let Err(err) = auto::Builder::new(TokioExecutor::new())
                            .serve_connection(
                                io,
                                service_fn(move |request: http::Request<hyper::body::Incoming>| {
                                    let topics = Arc::clone(&topics);
                                    async move {
                                        let (parts, body) = request.into_parts();
                                        let request = http::Request::from_parts(
                                            parts,
                                            body.collect().await?.to_bytes(),
                                        );

                                        let topics = topics.as_ref();
                                        let request =
                                            Request::from_http_with_context(&request, &topics);

                                        dbg!(&request.segments);

                                        Ok::<_, hyper::Error>(
                                            Router::route(&request)
                                                .await
                                                .unwrap_or_else(|| todo!("{:?}", request.request)),
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
