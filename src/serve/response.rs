use askama::{DynTemplate, Template};
use rocket::http::Status;
use rocket::response::{self, Responder};
use rocket::Request;
use std::fmt::Display;

pub const STYLE_CSS: &str = include_str!("../../dist/style.css");
pub const INIT_JS: &str = include_str!("../../dist/init.js");

pub enum Response<T> {
    Partial(Status, T),
    Page(Status, T),
}

impl<T> Response<T> {
    pub fn partial(response: T) -> Self {
        Self::Partial(Status::Ok, response)
    }

    pub fn page(template: T) -> Self {
        Self::Page(Status::Ok, template)
    }
}

#[derive(Template)]
#[template(path = "page.html", escape = "none")]
pub struct Page<T: Template> {
    pub template: T,
}

#[rocket::async_trait]
impl<'r, T> Responder<'r, 'static> for Response<T>
where
    T: Template,
{
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        match self {
            Response::Partial(status, template) => template.respond(status),
            Response::Page(status, template) => Page { template }.respond(status),
        }
    }
}

pub trait RocketTemplateExt {
    fn respond(self, status: Status) -> response::Result<'static>;
}

impl<T> RocketTemplateExt for T
where
    T: DynTemplate,
{
    fn respond(self, status: Status) -> response::Result<'static> {
        let response = self.dyn_render().map_err(|_| Status::InternalServerError)?;
        rocket::Response::build()
            .raw_header("Content-Type", self.mime_type())
            .sized_body(response.len(), std::io::Cursor::new(response))
            .status(status)
            .ok()
    }
}

pub enum Either<T, U> {
    A(T),
    B(U),
}

impl<T, U> Display for Either<T, U>
where
    T: Display,
    U: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A(data) => data.fmt(f),
            Self::B(data) => data.fmt(f),
        }
    }
}

impl<T, U> Template for Either<T, U>
where
    T: Template,
    U: Template,
{
    fn render_into(&self, writer: &mut (impl std::fmt::Write + ?Sized)) -> askama::Result<()> {
        match self {
            Self::A(template) => template.render_into(writer),
            Self::B(template) => template.render_into(writer),
        }
    }

    const EXTENSION: Option<&'static str> = T::EXTENSION;
    const SIZE_HINT: usize = T::SIZE_HINT;
    const MIME_TYPE: &'static str = T::MIME_TYPE;
}

impl<'r, T, U> Responder<'r, 'static> for Either<T, U>
where
    T: Responder<'r, 'static>,
    U: Responder<'r, 'static>,
{
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        match self {
            Self::A(responder) => responder.respond_to(request),
            Self::B(responder) => responder.respond_to(request),
        }
    }
}
