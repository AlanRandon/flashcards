use super::{flashcard, HxRequest, NodeExt, TopicQuery};
use crate::{Card, Topics};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Form,
};
use html_builder::prelude::*;
use rand::prelude::*;
use std::sync::Arc;

#[axum::debug_handler]
pub async fn get(
    Query(query): Query<TopicQuery>,
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    let Some(card) = get_random_card(&query, &state) else {
        return (
            StatusCode::NOT_FOUND,
            main()
                .class("grid place-items-center grow")
                .text("Set not found")
                .document_if(!is_htmx),
        )
            .into_response();
    };

    study(card.as_ref(), &query)
        .document_if(!is_htmx)
        .into_response()
}

#[axum::debug_handler]
pub async fn post(
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
    Form(query): Form<TopicQuery>,
) -> impl IntoResponse {
    match get_random_card(&query, &state) {
        Some(card) => if is_htmx {
            flashcard(&card).response()
        } else {
            study(&card, &query).document()
        }
        .into_response(),
        None => {
            if is_htmx {
                (
                    StatusCode::NOT_FOUND,
                    html::text("Card not found").response(),
                )
                    .into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    html::text("Card not found").document(),
                )
                    .into_response()
            }
        }
    }
}

fn study(card: &Card, query: &TopicQuery) -> Node {
    main()
        .class("grid place-items-center grow")
        .attr("hx-trigger", "keyup[key==' '] from:body")
        .attr("hx-on::trigger", "this.querySelector('.flashcard').click()")
        .child(
            form()
                .class("grid gap-4 [view-transition-name:study]")
                .attr("hx-post", "/study")
                .attr("hx-target", "find div")
                .attr(
                    "hx-trigger",
                    "submit, keyup[key=='Enter'&&!shiftKey] from:body",
                )
                .child(
                    input()
                        .attr("type", "hidden")
                        .attr("name", "name")
                        .attr("value", &query.name),
                )
                .child(
                    div()
                        .class("[view-transition-name:study-card]")
                        .child(flashcard(card)),
                )
                .child(
                    div().class("flex gap-4 items-center justify-center").child(
                        input()
                            .attr("type", "submit")
                            .attr("value", "Next")
                            .class("btn"),
                    ),
                ),
        )
        .into()
}

fn get_random_card(query: &TopicQuery, state: &Arc<Topics>) -> Option<Arc<Card>> {
    let mut rng = thread_rng();
    state.get(&query.name)?.choose(&mut rng).cloned()
}
