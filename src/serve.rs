use crate::render::filters;
use crate::{Card, Topics};
use askama::Template;
use auth::Authed;
use response::{Either, Response};
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::FromRequest;
use rocket::{catch, catchers, get, routes, FromForm, Request, State};
use std::convert::Infallible;

pub mod auth;
pub mod response;
mod study;
mod topic_list;

pub struct HxRequest(bool);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HxRequest {
    type Error = Infallible;

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = req.headers();
        Outcome::Success(Self(
            headers.get("Hx-Request").next().is_some()
                && !headers
                    .get("Hx-Trigger")
                    .next()
                    .is_some_and(|target| target == "auth-form"),
        ))
    }
}

#[derive(Debug, FromForm)]
pub struct TopicQuery<'r> {
    name: &'r str,
}

#[derive(Template)]
#[template(path = "view.html")]
struct View<'a> {
    cards: Vec<&'a Card>,
    name: &'a str,
}

#[derive(Template)]
#[template(
    source = r#"<main class="grid place-items-center grow"><p class="grow">Set not found</p></main>"#,
    ext = "html"
)]
struct SetNotFound;

#[get("/view?<query..>")]
fn view<'a>(
    query: TopicQuery<'a>,
    topics: &'a State<Topics>,
    htmx: HxRequest,
    _auth: Authed,
) -> Response<impl Template + 'a> {
    let Some(cards) = topics.get(query.name) else {
        return if htmx.0 {
            Response::Partial(Status::NotFound, Either::A(SetNotFound))
        } else {
            Response::Page(Status::NotFound, Either::A(SetNotFound))
        };
    };

    let cards = cards.iter().map(AsRef::as_ref).collect();
    let body = Either::B(View {
        cards,
        name: query.name,
    });

    if htmx.0 {
        Response::partial(body)
    } else {
        Response::page(body)
    }
}

#[derive(Template)]
#[template(
    source = r#"<main class="grid place-items-center grow"><h1>{{ err }}</h1></main>"#,
    ext = "html"
)]
struct Error {
    err: String,
}

#[catch(default)]
async fn catcher<'a>(
    status: Status,
    request: &'a Request<'_>,
) -> Either<Response<impl Template + 'a>, Response<impl Template>> {
    let Outcome::Success(_) = request.guard::<Authed>().await else {
        return Either::A(auth::catch_unauthorized(request));
    };

    let message = if status == Status::NotFound {
        format!("Page {} not found", request.uri())
    } else {
        "Unknown Error Occurred".to_string()
    };

    let htmx = HxRequest::from_request(request).await.unwrap();

    if htmx.0 {
        Either::B(Response::partial(Error { err: message }))
    } else {
        Either::B(Response::page(Error { err: message }))
    }
}

pub fn app(digest: auth::Digest, topics: Topics) -> rocket::Rocket<rocket::Build> {
    const STATIC_FILES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/dist/static");
    let static_files = rocket::fs::FileServer::from(STATIC_FILES);

    rocket::build()
        .mount(
            "/",
            routes![
                view,
                topic_list::index,
                topic_list::search,
                study::study_flashcard,
                study::study,
                auth::login,
            ],
        )
        .mount("/static", static_files)
        .register("/", catchers![auth::catch_unauthorized, catcher])
        .manage(topics)
        .manage(digest)
}
