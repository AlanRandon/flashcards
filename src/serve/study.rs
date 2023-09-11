use super::{flashcard, HxRequest, NodeExt, TopicQuery};
use crate::{Card, Topics};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse},
};
use html_builder::prelude::*;
use rand::prelude::*;
use std::sync::Arc;

#[axum::debug_handler]
pub async fn get(
    Query(query): Query<TopicQuery>,
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(card) = get_random_card(&query, &state) else {
        return (
            StatusCode::NOT_FOUND,
            main()
                .attr("hx-boost", true)
                .class("grid place-items-center grow")
                .text("Card not found")
                .document_if(!is_htmx),
        )
            .into_response();
    };

    let response = if headers.contains_key("Flashcards-Single-Card") {
        flashcard(card.as_ref()).response().into_response()
    } else {
        study(card.as_ref(), &query)
            .document_if(!is_htmx)
            .into_response()
    };

    (AppendHeaders([("Cache-Control", "no-cache")]), response).into_response()
}

fn study(card: &Card, query: &TopicQuery) -> Node {
    main()
        .class("flex grow")
        .attr("hx-trigger", "keyup[key==' '] from:body")
        .attr("hx-on::trigger", "this.querySelector('.flashcard').click()")
        .child(
            form()
                .class(
                    "flex justify-content-center flex-col gap-4 [view-transition-name:study] grow p-4",
                )
                .attr("method", "get")
                .attr("action", format!("study?name={}", query.name))
                .attr("hx-boost", true)
                .attr("hx-headers", r#"{"Flashcards-Single-Card":true}"#)
                .attr("hx-target", "find div")
                .attr(
                    "hx-trigger",
                    "submit, keyup[key=='Enter'&&!shiftKey] from:body",
                )
                .child(
                    div()
                        .class("[view-transition-name:study-card] grow flashcard-stretch")
                        .child(flashcard(card)),
                )
                .child(
                    div()
                        .class("flex gap-4 items-center justify-center")
                        .child(button().attr("type", "submit").class("btn").text("Next")),
                ),
        )
        .into()
}

fn get_random_card(query: &TopicQuery, state: &Arc<Topics>) -> Option<Arc<Card>> {
    let mut rng = thread_rng();
    state.get(&query.name)?.choose(&mut rng).cloned()
}
