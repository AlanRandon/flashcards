use crate::serve::auth::Authed;
use crate::serve::response::Response;
use crate::serve::{HxRequest, TopicQuery};
use crate::{Card, Topics};
use html_builder::prelude::*;
use rand::prelude::*;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::FromRequest;
use rocket::response::{self, Responder};
use rocket::{get, Request, State};
use std::convert::Infallible;

struct NoCache<T>(T);

impl<'r, T> Responder<'r, 'static> for NoCache<T>
where
    T: Responder<'r, 'static>,
{
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        response::Response::build_from(self.0.respond_to(req)?)
            .raw_header("Cache-Control", "no-cache")
            .ok()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StudyRequest {
    SingleFlashcard,
    StudyPage,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for StudyRequest {
    type Error = Infallible;

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = req.headers();
        let single = headers
            .get("Flashcards-Single-Card")
            .next()
            .is_some_and(|value| value == "true");

        Outcome::Success(if single {
            Self::SingleFlashcard
        } else {
            Self::StudyPage
        })
    }
}

#[get("/study?<query..>")]
pub async fn study(
    query: TopicQuery<'_>,
    state: &State<Topics>,
    htmx: HxRequest,
    request: StudyRequest,
    _auth: Authed,
) -> NoCache<Response> {
    let Some(card) = get_random_card(&query, &state) else {
        if request == StudyRequest::SingleFlashcard {
            return NoCache(Response::Partial(
                Status::NotFound,
                html::text("Card not found"),
            ));
        }

        let body = main()
            .attr("hx-boost", true)
            .class("grid place-items-center grow")
            .text("Card not found")
            .into();

        if htmx.0 {
            return NoCache(Response::Partial(Status::NotFound, body));
        }

        return NoCache(Response::Page(Status::NotFound, body));
    };

    match request {
        StudyRequest::StudyPage => {
            let main = study_page(&card, &query);
            if htmx.0 {
                NoCache(Response::partial(main))
            } else {
                NoCache(Response::page(main))
            }
        }
        StudyRequest::SingleFlashcard => NoCache(Response::partial(flashcard(&card))),
    }
}

fn study_page(card: &Card, query: &TopicQuery) -> Node {
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
                .attr("action", "/study")
                .attr("hx-boost", true)
                .attr("hx-headers", r#"{"Flashcards-Single-Card":true}"#)
                .attr("hx-target", "find div")
                .attr("hx-swap", "outerHTML")
                .attr(
                    "hx-trigger",
                    "submit, keyup[key=='Enter'&&!shiftKey] from:body",
                )
                .child(input().attr("type", "hidden").attr("name", "name").attr("value", &query.name))
                .child(flashcard(card))
                .child(
                    div()
                        .class("flex gap-4 items-center justify-center")
                        .child(button().attr("type", "submit").class("btn").text("Next")),
                ),
        )
        .into()
}

fn get_random_card<'a>(query: &TopicQuery<'_>, state: &'a Topics) -> Option<&'a Card> {
    let mut rng = thread_rng();
    state.get(query.name)?.choose(&mut rng).map(AsRef::as_ref)
}

fn flashcard(card: &Card) -> Node {
    div()
        .class("flex flex-col gap-4 [view-transition-name:study-card] grow flashcard-stretch")
        .child(super::flashcard(card))
        .child(
            div()
                .class("flex gap-4 items-center justify-center flex-wrap")
                .children(card.topics.iter().map(|topic| {
                    a().href(format!("/view/?name={topic}"))
                        .attr("hx-target", "main")
                        .attr("hx-swap", "outerHTML show:window:top")
                        .class("btn text-xs")
                        .text(topic)
                })),
        )
        .into()
}
