use super::{response, Request, RequestExt, Response};
use askama::Template;
use http::{Method, StatusCode, Uri};
use serde::Deserialize;
use std::future::Future;

#[derive(Debug, Deserialize)]
pub struct LoginBody<'r> {
    password: &'r str,
    #[serde(with = "http_serde::uri")]
    location: Uri,
}

pub async fn login<'a, F: Future<Output = Response>>(
    request: &'a Request<'a>,
    on_authed: impl FnOnce(&'a Request<'a>) -> F,
) -> Response {
    let cookie = request
        .request
        .headers()
        .get("Cookie")
        .and_then(|header| header.to_str().ok())
        .map(cookie::Cookie::split_parse_encoded)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .find(|cookie| cookie.name() == "auth");

    let cookie = cookie.as_ref().map(cookie::Cookie::value);

    if request.segments == ["login"] {
        if request.request.method() == Method::POST {
            if let Ok(LoginBody { password, location }) = request.form().await {
                if password == request.context.password.as_ref() {
                    return http::Response::builder()
                        .status(StatusCode::SEE_OTHER)
                        .header("Set-Cookie", format!("auth={password}; Http-Only; Secure;"))
                        .header("Location", location.to_string())
                        .body(http_body_util::Full::default())
                        .unwrap();
                };
            }
        }

        let location = match request.query::<LoginFormParams>() {
            Ok(query) => query.location,
            Err(_) => Uri::from_static("/"),
        };

        if Some(request.context.password.as_ref()) == cookie {
            return http::Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header("Location", location.to_string())
                .body(http_body_util::Full::default())
                .unwrap();
        }

        let mut response = response::partial(
            &LoginForm {
                location: &location,
            },
            StatusCode::UNAUTHORIZED,
        );
        response
            .headers_mut()
            .append("HX-Refresh", http::HeaderValue::from_static("true"));
        return response;
    }

    if Some(request.context.password.as_ref()) == cookie {
        return on_authed(request).await;
    }

    http::Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(
            "Location",
            format!("login?location={}", request.request.uri()),
        )
        .body(http_body_util::Full::default())
        .unwrap()
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginForm<'a> {
    location: &'a Uri,
}

#[derive(Deserialize)]
struct LoginFormParams {
    #[serde(with = "http_serde::uri")]
    location: Uri,
}
