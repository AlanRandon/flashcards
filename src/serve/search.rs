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
    let topics = topics.0.keys().map(AsRef::as_ref);

    let topics = if query.q.is_empty() {
        topics.collect()
    } else {
        let query = nucleo_matcher::pattern::Pattern::parse(
            query.q,
            nucleo_matcher::pattern::CaseMatching::Smart,
        );
        let mut matcher = nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT);

        topics
            .map(|topic| (topic, nucleo_matcher::Utf32String::from(topic)))
            .sorted_by(|(_, a), (_, b)| {
                query
                    .score(b.slice(..), &mut matcher)
                    .cmp(&query.score(a.slice(..), &mut matcher))
            })
            .map(|(topic, _)| topic)
            .collect()
    };

    if htmx.0 {
        Response::partial(Search { topics })
    } else {
        Response::page(Search { topics })
    }
}
