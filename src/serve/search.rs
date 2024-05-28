use super::{response, Error, Request, RequestExt, Response};
use crate::Topic;
use askama::Template;
use http::StatusCode;
use itertools::Itertools;
use router::prelude::*;
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Template)]
#[template(path = "search.html")]
struct Search<'a> {
    topics: Vec<&'a Topic>,
}

#[get]
pub fn index(request: &Request<'req>) -> Response {
    let topics = request.context.topics.topics.keys().collect();

    response::partial_if(&Search { topics }, StatusCode::OK, request.is_htmx())
}

#[derive(Debug, Deserialize)]
pub struct Query<'r> {
    q: Cow<'r, str>,
}

#[post("search")]
pub async fn search(request: &Request<'req>) -> Response {
    let topics = request.context.topics.topics.keys();

    let Ok(body) = request.form::<Query>().await else {
        return response::partial_if(
            &Error {
                err: "Invalid form body",
            },
            StatusCode::BAD_REQUEST,
            request.is_htmx(),
        );
    };

    let topics = if body.q.is_empty() {
        topics.collect_vec()
    } else {
        // let split = |ch: char| ch.is_whitespace() || ch == '/';
        // let query_parts = body.q.split(split).collect::<Vec<_>>();
        // topic.split(split).filter_map(|part| {
        //     query_parts.map(|query_part|)
        // }),

        let match_score = |topic: &Topic| {
            topic
                .0
                .iter()
                .fold(None, |score, segment| {
                    let score = match (
                        score,
                        sublime_fuzzy::best_match(&body.q, segment).map(|score| score.score()),
                    ) {
                        (Some(a), Some(b)) => a + b as f32,
                        (Some(score), None) => score,
                        (None, Some(score)) => score as f32,
                        (None, None) => return None,
                    };

                    Some(score)
                })
                .map(|score| score / topic.0.len() as f32)
        };

        topics
            .filter_map(|topic| match_score(topic).map(|score| (topic, score)))
            .sorted_by(|(_, score_a), (_, score_b)| {
                score_b
                    .partial_cmp(score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(topic, _)| topic)
            .collect_vec()
    };

    response::partial_if(&Search { topics }, StatusCode::OK, request.is_htmx())
}
