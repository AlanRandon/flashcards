use super::{response, Error, Request, RequestExt, Response};
use askama::Template;
use http::StatusCode;
use itertools::Itertools;
use router::prelude::*;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "search.html")]
struct Search<'a> {
    topics: Vec<&'a str>,
}

#[get("")]
pub fn index(request: &Request<'req>) -> Response {
    let topics = request.context.0.keys().map(AsRef::as_ref).collect();
    response::partial_if(Search { topics }, StatusCode::OK, request.is_htmx())
}

#[derive(Debug, Deserialize)]
pub struct Query<'a> {
    q: &'a str,
}

#[post("search")]
pub async fn search(request: &Request<'req>) -> Response {
    let topics = request.context.0.keys().map(AsRef::as_ref);

    let Ok(body) = request.form::<Query>().await else {
        return response::partial_if(
            Error {
                err: "Invalid form body",
            },
            StatusCode::BAD_REQUEST,
            request.is_htmx(),
        );
    };

    let topics = if body.q.is_empty() {
        topics.collect()
    } else {
        let query = nucleo_matcher::pattern::Pattern::parse(
            body.q,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Smart,
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

    response::partial_if(Search { topics }, StatusCode::OK, request.is_htmx())
}
