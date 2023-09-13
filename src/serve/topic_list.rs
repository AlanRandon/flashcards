use crate::serve::auth::Authed;
use crate::serve::response::Response;
use crate::serve::HxRequest;
use crate::{Card, Topics};
use html_builder::prelude::*;
use rocket::form::Form;
use rocket::{get, post, FromForm, State};
use std::sync::Arc;

fn search_form(topics: &Topics, filter: impl Fn(&str, &[Arc<Card>]) -> bool) -> Node {
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
        .child(topic_list(topics, filter))
        .into()
}

fn topic_list(topics: &Topics, filter: impl Fn(&str, &[Arc<Card>]) -> bool) -> Node {
    div()
        .id("topic-list")
        .class("auto-grid-[25ch] gap-4")
        .children(
            topics
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

#[get("/")]
pub fn index(topics: &State<Topics>, htmx: HxRequest, _auth: Authed) -> Response {
    let filter = |_: &_, _: &_| true;
    if htmx.0 {
        Response::partial(search_form(topics, filter))
    } else {
        Response::page(search_form(topics, filter))
    }
}

#[derive(Debug, FromForm)]
pub struct SearchQuery<'r> {
    q: &'r str,
}

#[post("/search", data = "<query>")]
pub fn search(
    query: Form<SearchQuery<'_>>,
    state: &State<Topics>,
    htmx: HxRequest,
    _auth: Authed,
) -> Response {
    let filter = |name: &str, _: &_| name.contains(query.q);
    if htmx.0 {
        Response::partial(topic_list(state, filter))
    } else {
        Response::page(search_form(state, filter))
    }
}
