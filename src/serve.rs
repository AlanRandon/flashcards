use crate::Topics;
use axum::{
    body::Body,
    extract::{Query, State},
    http::{Request, StatusCode},
    response::Html,
    routing::get,
    Router,
};
use html_builder::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

trait NodeExt {
    fn document(self) -> Html<String>;
    fn response(self) -> Html<String>;
}

impl<T> NodeExt for T
where
    Node: From<T>,
{
    fn document(self) -> Html<String> {
        Html(
            html::document::<Node, Node>(
                [
                    style()
                        .child(html_builder::raw_text(include_str!("../dist/style.css")))
                        .into(),
                    title().text("App").into(),
                ],
                [
                    self.into(),
                    script()
                        .child(html_builder::raw_text(include_str!("../dist/init.js")))
                        .into(),
                ],
            )
            .to_string(),
        )
    }

    fn response(self) -> Html<String> {
        Html(Node::from(self).to_string())
    }
}

pub async fn serve(state: Topics) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(state);

    #[derive(Debug, Deserialize)]
    struct TopicQuery {
        name: String,
    }

    let app = Router::new()
        .route(
            "/",
            get(|State(state): State<Arc<Topics>>| async move {
                form()
                    .attr("hx-get", "/topic")
                    .attr("hx-trigger", "input from:input")
                    .attr("hx-target", "div")
                    .child(input().attr("list", "topics").attr("name", "name"))
                    .child(
                        datalist()
                            .id("topics")
                            .children(state.0.keys().map(|name| option().attr("value", name))),
                    )
                    .child(div())
                    .document()
            }),
        )
        .route(
            "/topic",
            get(
                |Query(query): Query<TopicQuery>, State(state): State<Arc<Topics>>| async move {
                    div()
                        .children(state.get(&query.name).into_iter().flatten().map(|card| {
                            div()
                                .child(div().text(&card.term))
                                .child(div().text(&card.definition))
                        }))
                        .response()
                },
            ),
        )
        .fallback(|req: Request<Body>| async move {
            (
                StatusCode::NOT_FOUND,
                h1().text(format!("Page {} not found", req.uri()))
                    .document(),
            )
        })
        .with_state(state);

    let addr = "127.0.0.1:8000".parse()?;

    Ok(axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?)
}
