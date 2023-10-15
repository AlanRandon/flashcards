use crate::render::filters;
use crate::serve::auth::Authed;
use crate::serve::response::Response;
use crate::serve::{HxRequest, TopicQuery};
use crate::{Card, Topics};
use askama::Template;
use rand::prelude::*;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::FromRequest;
use rocket::response::{self, Responder};
use rocket::{get, Request, State};

use super::response::Either;

pub struct NoCache<T>(T);

impl<'r, T> Responder<'r, 'static> for NoCache<T>
where
    T: Responder<'r, 'static>,
{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        response::Response::build_from(self.0.respond_to(req)?)
            .raw_header("Cache-Control", "no-cache")
            .ok()
    }
}

#[derive(Template)]
#[template(
    source = r#"<main class="grid place-items-center grow"><p class="grow">Card not found</p></main>"#,
    ext = "html"
)]
struct CardNotFound;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SingleFlashcard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SingleFlashcard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = req.headers();
        let header = headers.get("Flashcards-Single-Card").next();
        if header.is_some_and(|value| value == "true") {
            Outcome::Success(Self)
        } else {
            Outcome::Forward(())
        }
    }
}

#[get("/study?<query..>", rank = 2)]
pub fn study<'a>(
    query: TopicQuery<'a>,
    state: &'a State<Topics>,
    htmx: HxRequest,
    _auth: Authed,
) -> NoCache<Response<impl Template + 'a>> {
    let Some(card) = get_random_card(&query, state) else {
        return if htmx.0 {
            NoCache(Response::Partial(Status::NotFound, Either::A(CardNotFound)))
        } else {
            NoCache(Response::Partial(Status::NotFound, Either::A(CardNotFound)))
        };
    };

    let body = Either::B(Study {
        card,
        name: query.name,
    });
    if htmx.0 {
        NoCache(Response::partial(body))
    } else {
        NoCache(Response::page(body))
    }
}

#[get("/study?<query..>")]
pub fn study_flashcard<'a>(
    query: TopicQuery<'a>,
    state: &'a State<Topics>,
    _single: SingleFlashcard,
    _auth: Authed,
) -> NoCache<Response<impl Template + 'a>> {
    let Some(card) = get_random_card(&query, state) else {
        return NoCache(Response::Page(Status::NotFound, Either::A(CardNotFound)));
    };

    NoCache(Response::partial(Either::B(Flashcard { card })))
}

fn get_random_card<'a>(query: &TopicQuery<'_>, state: &'a Topics) -> Option<&'a Card> {
    let mut rng = thread_rng();
    state.get(query.name)?.choose(&mut rng).map(AsRef::as_ref)
}

#[derive(Template)]
#[template(path = "study.html")]
struct Study<'a> {
    card: &'a Card,
    name: &'a str,
}

#[derive(Template)]
#[template(path = "study_flashcard.html")]
struct Flashcard<'a> {
    card: &'a Card,
}
