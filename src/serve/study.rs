use crate::render::filters;
use crate::serve::auth::Authed;
use crate::serve::TopicQuery;
use crate::{Card, Topics};
use askama::Template;
use rand::prelude::*;

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

#[get("/study?<query..>")]
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
            NoCache(Response::Page(Status::NotFound, Either::A(CardNotFound)))
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
