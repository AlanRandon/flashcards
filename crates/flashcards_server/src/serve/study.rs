use super::{Error, RenderedCard, Request, RequestExt, Response, response};
use crate::collection::deserialize::Topic;
use askama::Template;
use http::StatusCode;
use rand::prelude::*;
use router::prelude::*;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Deserialize, Serialize)]
pub struct Query<'r> {
    name: Cow<'r, str>,
    id: Option<usize>,
}

#[get("study")]
pub fn study(request: &Request<'req>) -> Response {
    let Ok(mut query) = request.query::<Query>() else {
        return response::no_cache(response::partial_if(
            &Error {
                err: "Invalid query",
            },
            StatusCode::BAD_REQUEST,
            request.is_htmx(),
        ));
    };

    let Some(topic) = request.context.topics.get(&Topic::new(&query.name)) else {
        return response::no_cache(response::partial_if(
            &Error {
                err: "Topic not found",
            },
            StatusCode::NOT_FOUND,
            request.is_htmx(),
        ));
    };

    let id = query.id.unwrap_or_else(|| {
        let mut rng = rand::rng();
        let id = rand::distr::Uniform::new(0, topic.len())
            .unwrap()
            .sample(&mut rng);

        query.id = Some(id);
        id
    });

    let Some(card) = topic.get(id).map(AsRef::as_ref) else {
        return response::no_cache(response::partial_if(
            &Error {
                err: "Card not found",
            },
            StatusCode::NOT_FOUND,
            request.is_htmx(),
        ));
    };

    response::header(
        response::no_cache(response::partial_if(
            &Study {
                card,
                topic: &query.name,
                previous_card_id: id.checked_sub(1),
                next_card_id: id.checked_add(1).filter(|id| *id < topic.len()),
            },
            StatusCode::OK,
            request.is_htmx(),
        )),
        "HX-Push-Url",
        &format!(
            "/study?{}",
            &serde_urlencoded::to_string(query).unwrap_or_else(|_| "false".to_string())
        ),
    )
}

#[derive(Template)]
#[template(path = "study.html")]
struct Study<'a> {
    card: &'a RenderedCard,
    topic: &'a str,
    next_card_id: Option<usize>,
    previous_card_id: Option<usize>,
}
