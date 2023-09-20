use html_builder::prelude::*;
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::response::{self, Responder};
use rocket::Request;

pub static mut STYLE_CSS: String = String::new();
pub static mut INIT_JS: String = String::new();

pub enum Response {
    Partial(Status, Node),
    Page(Status, Node),
    Document(Status, Node),
}

impl Response {
    pub fn partial<T>(node: T) -> Self
    where
        Node: From<T>,
    {
        Self::Partial(Status::Ok, node.into())
    }

    pub fn page<T>(node: T) -> Self
    where
        Node: From<T>,
    {
        Self::Page(Status::Ok, node.into())
    }

    #[allow(dead_code)]
    pub fn document<T>(node: T) -> Self
    where
        Node: From<T>,
    {
        Self::Document(Status::Ok, node.into())
    }
}

fn document<I>(items: I) -> RawHtml<String>
where
    I: IntoIterator<Item = Node>,
{
    RawHtml(
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
            items.into_iter().chain(std::iter::once(Node::Element(
                script().child(html_builder::raw_text(unsafe { &INIT_JS })),
            ))),
        )
        .to_string(),
    )
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Response {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        match self {
            Self::Partial(status, node) => (status, RawHtml(node.to_string())).respond_to(request),
            Self::Page(status, node) => (status, {
                const NAV_CLASSES: &str =
        "bg-slate-100 shadow rounded-b p-4 sticky top-0 z-10 [view-transition-name:nav] h-fit";

                document([
                    Node::Element(
                        nav().class(NAV_CLASSES).attr("hx-boost", true).child(
                            a().href("/")
                                .class("font-bold")
                                .text("Flashcards")
                                .attr("hx-target", "main")
                                .attr("hx-swap", "outerHTML show:window:top"),
                        ),
                    ),
                    node,
                ])
            })
                .respond_to(request),
            Self::Document(status, node) => (status, document([node])).respond_to(request),
        }
    }
}
