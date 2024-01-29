use super::{response, Error, Request, RequestExt, Response};
use crate::render::filters;
use crate::serve::TopicQuery;
use crate::{Card, Topics};
use askama::Template;
use http::StatusCode;
use rand::prelude::*;
use router::prelude::*;

#[get("study")]
pub fn study(request: &Request<'req>) -> Response {
    let Ok(query) = request.query::<TopicQuery>() else {
        return response::no_cache(response::partial_if(
            &Error {
                err: "Invalid query",
            },
            StatusCode::BAD_REQUEST,
            request.is_htmx(),
        ));
    };

    let Some(card) = get_random_card(&query, request.context.topics) else {
        return response::no_cache(response::partial_if(
            &Error {
                err: "Card not found",
            },
            StatusCode::NOT_FOUND,
            request.is_htmx(),
        ));
    };

    response::no_cache(response::partial_if(
        &Study {
            card,
            name: query.name,
        },
        StatusCode::NOT_FOUND,
        request.is_htmx(),
    ))
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
