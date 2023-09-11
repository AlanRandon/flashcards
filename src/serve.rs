use crate::{Card, Topics};
use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, Query, State},
    http::{request::Parts, Request, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Form, Router, ServiceExt,
};
use html_builder::prelude::*;
use serde::Deserialize;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tower_http::normalize_path::NormalizePath;

pub mod auth;
pub mod study;

pub trait NodeExt {
    fn response(self) -> Html<String>;
    fn raw_document<I>(items: I) -> Html<String>
    where
        I: Iterator<Item = Node>;
    fn document(self) -> Html<String>;

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

pub static mut STYLE_CSS: String = String::new();
pub static mut INIT_JS: String = String::new();

impl<T> NodeExt for T
where
    Node: From<T>,
{
    fn raw_document<I>(items: I) -> Html<String>
    where
        I: Iterator<Item = Node>,
    {
        Html(
            html::document::<Node, Node>(
                [
                    meta()
                        .attr("name", "htmx-config")
                        .attr("content", r#"{"globalViewTransitions":true}"#)
                        .into(),
                    style()
                        .child(html_builder::raw_text(unsafe { &STYLE_CSS }))
                        .into(),
                    title().text("App").into(),
                ],
                items.chain(std::iter::once(Node::Element(
                    script().child(html_builder::raw_text(unsafe { &INIT_JS })),
                ))),
            )
            .to_string(),
        )
    }

    fn document(self) -> Html<String> {
        const NAV_CLASSES: &str =
            "bg-slate-100 shadow rounded-b p-4 sticky top-0 z-10 [view-transition-name:nav] h-fit";
        Self::raw_document::<<[Node; 2] as IntoIterator>::IntoIter>(
            [
                Node::Element(
                    nav().class(NAV_CLASSES).attr("hx-boost", true).child(
                        a().href("/")
                            .class("font-bold")
                            .text("Flashcards")
                            .attr("hx-target", "main")
                            .attr("hx-swap", "outerHTML show:window:top"),
                    ),
                ),
                self.into(),
            ]
            .into_iter(),
        )
    }

    fn response(self) -> Html<String> {
        Html(Node::from(self).to_string())
    }
}

fn markdown(text: &str) -> Node {
    let mut result = String::new();
    pulldown_cmark::html::push_html(
        &mut result,
        pulldown_cmark::Parser::new_ext(text, pulldown_cmark::Options::ENABLE_TABLES),
    );
    html::Node::RawHtml(result)
}

fn flashcard(card: &Card) -> Node {
    div()
        .class("flashcard")
        .attr("hx-on:click", "this.classList.toggle('flashcard-flipped')")
        .child(
            div()
                .attr("hx-ignore", true)
                .class("flashcard-side prose prose-slate")
                .child(markdown(&card.term)),
        )
        .child(
            div()
                .attr("hx-ignore", true)
                .class("flashcard-side prose prose-slate")
                .child(markdown(&card.definition)),
        )
        .into()
}

pub struct HxRequest(bool);

#[async_trait]
impl<S> FromRequestParts<S> for HxRequest
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts.headers.get("Hx-Request").is_some()
                && !parts
                    .headers
                    .get("Hx-Trigger")
                    .is_some_and(|target| matches!(target.to_str(), Ok("auth-form"))),
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct TopicQuery {
    name: String,
}

pub async fn view(
    Query(query): Query<TopicQuery>,
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    let cards = match state.get(&query.name) {
        Some(cards) => cards.iter().map(|card| flashcard(card.as_ref())),
        None => {
            return (
                StatusCode::NOT_FOUND,
                main()
                    .class("grid place-items-center grow")
                    .child(p().class("grow").text("Set not found"))
                    .document_if(!is_htmx),
            )
                .into_response();
        }
    };

    let study = format!("/study?name={}", query.name);

    main()
        .attr("hx-boost", true)
        .class("auto-grid-[25ch] gap-4 p-4")
        .child(
            div()
                .class("col-span-full grid place-items-center gap-4")
                .child(h1().class("text-xl font-bold").text(&query.name))
                .text(format!("{} cards", cards.len()))
                .child(
                    a().href(study)
                        .text("Study")
                        .attr("hx-target", "main")
                        .attr("hx-swap", "outerHTML show:window:top")
                        .class("btn"),
                ),
        )
        .children(cards)
        .document_if(!is_htmx)
        .into_response()
}

fn search_form(state: Arc<Topics>, filter: impl Fn(&str, &[Arc<Card>]) -> bool) -> Node {
    const INPUT_CLASSES: &str = "border-slate-200 border-2 rounded-[100vmax] focus-within:border-slate-200 transition-colors p-2";

    main()
        .attr("hx-boost", true)
        .class("p-4 grid gap-4")
        .child(
            form()
                .class("w-full grid place-items-center gap-4 [view-transition-name:search]")
                .attr("action", "/search")
                .attr("method", "post")
                .attr("hx-target", "#topic-list")
                .attr("hx-swap", "outerHTML show:window:top")
                .attr(
                    "hx-trigger",
                    "keyup changed delay:300ms from:find input, submit",
                )
                .attr("hx-push-url", false)
                .child(h2().text("Search"))
                .child(
                    div()
                        .class("flex gap-4 items-center justify-center flex-wrap")
                        .child(
                            input()
                                .attr("type", "search")
                                .attr("name", "q")
                                .class(INPUT_CLASSES),
                        )
                        .child(
                            noscript()
                                .child(button().attr("type", "submit").class("btn").text("Go")),
                        ),
                ),
        )
        .child(topic_list(state, filter))
        .into()
}

fn topic_list(state: Arc<Topics>, filter: impl Fn(&str, &[Arc<Card>]) -> bool) -> Node {
    div()
        .id("topic-list")
        .class("auto-grid-[25ch] gap-4")
        .children(
            state
                .0
                .iter()
                .filter(|(topic, cards)| filter(topic, cards))
                .map(|(topic, _)| {
                    div()
                        .class(
                            "bg-slate-100 shadow rounded p-4 flex flex-col gap-4 justify-between",
                        )
                        .child(h2().text(topic))
                        .child({
                            let view = format!("/view?name={topic}");
                            let study = format!("/study?name={topic}");

                            div()
                                .id(format!("topic-{topic}"))
                                .class("flex gap-4 justify-end")
                                .child(
                                    a().href(view)
                                        .text("View")
                                        .attr("hx-target", "main")
                                        .attr("hx-swap", "outerHTML show:window:top")
                                        .class("btn"),
                                )
                                .child(
                                    a().href(study)
                                        .text("Study")
                                        .attr("hx-target", "main")
                                        .attr("hx-swap", "outerHTML show:window:top")
                                        .class("btn"),
                                )
                        })
                }),
        )
        .into()
}

pub async fn index(
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
) -> impl IntoResponse {
    search_form(state, |_, _| true).document_if(!is_htmx)
}

#[derive(Debug, Deserialize)]
pub struct SearchBody {
    q: String,
}

#[axum::debug_handler]
pub async fn search(
    State(state): State<Arc<Topics>>,
    HxRequest(is_htmx): HxRequest,
    Form(query): Form<SearchBody>,
) -> impl IntoResponse {
    let filter = |name: &str, _: &_| name.contains(&query.q);
    if is_htmx {
        topic_list(state, filter).response()
    } else {
        search_form(state, filter).document()
    }
}

pub struct App {
    pub digest: auth::Digest,
    pub topics: Topics,
}

impl App {
    pub async fn bind(self, addr: &SocketAddr) {
        let app = Router::new()
            .route("/", get(index))
            .route("/search", post(search))
            .route("/view", get(view))
            .route("/study", get(study::get))
            .fallback(|req: Request<Body>| async move {
                (
                    StatusCode::NOT_FOUND,
                    html::main()
                        .attr("hx-boost", true)
                        .class("grid place-items-center grow")
                        .child(h1().text(format!("Page {} not found", req.uri())))
                        .document(),
                )
            })
            .with_state(Arc::new(self.topics));

        let app = NormalizePath::trim_trailing_slash(app);

        let app = auth::Auth::new(app, self.digest);

        axum::Server::bind(addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}
