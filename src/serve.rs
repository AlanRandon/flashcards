use crate::Topics;
use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, Query, State},
    http::{request::Parts, Request, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router, ServiceExt,
};
use html_builder::prelude::*;
use serde::Deserialize;
use std::{convert::Infallible, sync::Arc};
use tower_http::normalize_path::NormalizePath;

trait NodeExt {
    fn document(self) -> Html<String>;
    fn response(self) -> Html<String>;

    fn document_if(self, value: bool) -> Html<String>
    where
        Self: Sized,
    {
        if value {
            self.document()
        } else {
            self.response()
        }
    }
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
                    nav()
                        .class("bg-slate-100 shadow rounded-b p-4")
                        .child(
                            a().href("/")
                                .text("Flashcards")
                                .attr("hx-get", "/")
                                .attr("hx-target", "main")
                                .attr("hx-swap", "outerHTML")
                                .attr("hx-push-url", true),
                        )
                        .into(),
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

struct HxRequest(bool);

#[async_trait]
impl<S> FromRequestParts<S> for HxRequest
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.headers.get("Hx-Request").is_some()))
    }
}

#[derive(Debug, Deserialize)]
struct TopicQuery {
    name: String,
}

async fn view(
    Query(query): Query<TopicQuery>,
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    main()
        .children(state.get(&query.name).into_iter().flatten().map(|card| {
            div()
                .child(div().text(&card.term))
                .child(div().text(&card.definition))
        }))
        .document_if(!is_htmx)
}

async fn study(
    Query(query): Query<TopicQuery>,
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    main().child(div()).document_if(!is_htmx)
}

async fn index(
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    main()
        .class("p-4 auto-grid-[25ch] gap-4")
        .children(state.0.keys().map(|topic| {
            let view = format!("/view?name={topic}");
            let study = format!("/study?name={topic}");

            div()
                .class("bg-slate-100 shadow rounded p-4 flex flex-col gap-4 justify-between")
                .child(h2().text(topic))
                .child(
                    div()
                        .class("flex gap-4 justify-end")
                        .child(
                            a().href(&view)
                                .text("View")
                                .attr("hx-get", view)
                                .attr("hx-target", "main")
                                .attr("hx-swap", "outerHTML")
                                .attr("hx-push-url", true),
                        )
                        .child(
                            a().href(&study)
                                .text("Study")
                                .attr("hx-get", study)
                                .attr("hx-target", "main")
                                .attr("hx-swap", "outerHTML")
                                .attr("hx-push-url", true),
                        ),
                )
        }))
        .document_if(!is_htmx)
}

pub async fn serve(state: Topics) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(state);

    let app = Router::new()
        .route("/", get(index))
        .route("/view", get(view))
        .route("/study", get(study))
        .fallback(|req: Request<Body>| async move {
            (
                StatusCode::NOT_FOUND,
                h1().text(format!("Page {} not found", req.uri()))
                    .document(),
            )
        })
        .with_state(state);

    let app = NormalizePath::trim_trailing_slash(app);

    let addr = "127.0.0.1:8000".parse()?;

    Ok(axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?)
}
