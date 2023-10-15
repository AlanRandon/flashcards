use crate::serve::auth::Authed;
use crate::serve::response::Response;
use crate::serve::HxRequest;
use crate::Topics;
use askama::Template;
use rocket::form::Form;
use rocket::{get, post, FromForm, State};

use super::response::Either;

#[derive(Template)]
#[template(path = "topic_list.html")]
struct TopicList<'a> {
    topics: Vec<&'a str>,
}

#[derive(Template)]
#[template(path = "search.html")]
struct Search<'a> {
    topics: Vec<&'a str>,
}

#[get("/")]
pub fn index(
    topics: &State<Topics>,
    htmx: HxRequest,
    _auth: Authed,
) -> Response<impl Template + '_> {
    let topics = topics.0.keys().map(AsRef::as_ref).collect();
    let body = Search { topics };
    if htmx.0 {
        Response::partial(body)
    } else {
        Response::page(body)
    }
}

#[derive(Debug, FromForm)]
pub struct SearchQuery<'r> {
    q: &'r str,
}

#[post("/search", data = "<query>")]
pub fn search<'a>(
    query: Form<SearchQuery<'_>>,
    topics: &'a State<Topics>,
    htmx: HxRequest,
    _auth: Authed,
) -> Response<impl Template + 'a> {
    let topics = topics
        .0
        .keys()
        .map(AsRef::as_ref)
        .filter(|name| name.contains(query.q))
        .collect();

    if htmx.0 {
        Response::partial(Either::A(TopicList { topics }))
    } else {
        Response::page(Either::B(Search { topics }))
    }
}
