use crate::{Card, Topics};
use auth::Authed;
use html_builder::prelude::*;
use response::Response;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::FromRequest;
use rocket::{catch, catchers, get, routes, FromForm, Request, State};
use std::convert::Infallible;

pub mod auth;
pub mod response;
mod study;
mod topic_list;

fn markdown(text: &str) -> Node {
    html::Node::RawHtml(crate::render::markdown(text))
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HxRequest {
    type Error = Infallible;

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = req.headers();
        Outcome::Success(Self(
            headers.get("Hx-Request").next().is_some()
                && !headers
                    .get("Hx-Trigger")
                    .next()
                    .is_some_and(|target| target == "auth-form"),
        ))
    }
}

#[derive(Debug, FromForm)]
pub struct TopicQuery<'r> {
    name: &'r str,
}

#[get("/view?<query..>")]
fn view(query: TopicQuery<'_>, topics: &State<Topics>, htmx: HxRequest, _auth: Authed) -> Response {
    let cards = match topics.get(&query.name) {
        Some(cards) => cards.iter().map(|card| flashcard(card.as_ref())),
        None => {
            let body = main()
                .class("grid place-items-center grow")
                .child(p().class("grow").text("Set not found"))
                .into();

            if htmx.0 {
                return Response::Partial(Status::NotFound, body);
            }

            return Response::Page(Status::NotFound, body);
        }
    };

    let study = format!("/study?name={}", query.name);

    let body: Node = main()
        .attr("hx-boost", true)
        .class("auto-grid-[25ch] gap-4 p-4")
        .child(
            div()
                .class("col-span-full grid place-items-center gap-4")
                .child(h1().class("text-xl font-bold").text(query.name))
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
        .into();

    if htmx.0 {
        Response::partial(body)
    } else {
        Response::page(body)
    }
}

#[catch(default)]
async fn catcher(status: Status, request: &Request<'_>) -> Response {
    let Outcome::Success(_) = request.guard::<Authed>().await else {
        return auth::catch_unauthorized(request);
    };

    let message = if status == Status::NotFound {
        format!("Page {} not found", request.uri())
    } else {
        "Unknown Error Occurred".to_string()
    };

    let message = html::main()
        .class("grid place-items-center grow")
        .child(h1().text(message));

    let htmx = HxRequest::from_request(request).await.unwrap();

    if htmx.0 {
        Response::partial(message)
    } else {
        Response::page(message)
    }
}

pub fn app(digest: auth::Digest, topics: Topics) -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .mount(
            "/",
            routes![
                view,
                topic_list::index,
                topic_list::search,
                study::study,
                auth::login
            ],
        )
        .register("/", catchers![auth::catch_unauthorized, catcher])
        .manage(topics)
        .manage(digest)
}
