use crate::serve::Response;
use askama::Template;
use http::StatusCode;
use std::fmt::Display;

pub const STYLE_CSS: &str = include_str!("../../dist/style.css");
pub const INIT_JS: &str = include_str!("../../dist/init.js");

#[derive(Template)]
#[template(path = "page.html", escape = "none")]
pub struct Page<T: Template> {
    pub template: T,
}

pub fn partial(template: impl Template, status: StatusCode) -> Response {
    use bytes::Bytes;
    use http_body_util::Full;

    let Ok(body) = template.render() else {
        return http::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Unknown error")))
            .unwrap();
    };

    http::Response::builder()
        .status(status)
        .body(http_body_util::Full::new(Bytes::from(body)))
        .unwrap()
}

pub fn page(template: impl Template, status: StatusCode) -> Response {
    partial(Page { template }, status)
}

pub fn partial_if(template: impl Template, status: StatusCode, condition: bool) -> Response {
    if condition {
        partial(template, status)
    } else {
        page(template, status)
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
