use crate::serve::Response;
use askama::{DynTemplate, Template};
use http::StatusCode;

pub const STYLE_CSS: &str = include_str!("../../dist/style.css");
pub const INIT_JS: &str = include_str!("../../dist/init.js");

#[derive(Template)]
#[template(path = "page.html", escape = "none")]
pub struct Page<'a, T: Template> {
    pub template: &'a T,
}

pub fn partial(template: &impl Template, status: StatusCode) -> Response {
    use bytes::Bytes;
    use http_body_util::Full;

    let Ok(body) = template.render() else {
        return http::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", template.mime_type())
            .body(Full::new(Bytes::from("Unknown error")))
            .unwrap();
    };

    http::Response::builder()
        .status(status)
        .header("Content-Type", template.mime_type())
        .body(http_body_util::Full::new(Bytes::from(body)))
        .unwrap()
}

pub fn page(template: &impl Template, status: StatusCode) -> Response {
    partial(&Page { template }, status)
}

pub fn partial_if(template: &impl Template, status: StatusCode, condition: bool) -> Response {
    if condition {
        partial(template, status)
    } else {
        page(template, status)
    }
}

pub fn no_cache(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert("Cache-Control", http::HeaderValue::from_static("no-cache"));
    response
}
