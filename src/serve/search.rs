use crate::serve::auth::Authed;
use crate::serve::response::Response;
use crate::serve::HxRequest;
use crate::Topics;
use askama::Template;
use itertools::Itertools;
use rocket::form::Form;
use rocket::{get, post, FromForm, State};

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
    let words = query.q.split_whitespace().collect_vec();

    let topics = topics.0.keys().map(AsRef::as_ref);

    let topics = if words.is_empty() {
        topics.collect()
    } else {
        topics
            .filter(|name| words.iter().any(|word| name.contains(word)))
            .collect()
    };

    if htmx.0 {
        Response::partial(Search { topics })
    } else {
        Response::page(Search { topics })
    }
}
