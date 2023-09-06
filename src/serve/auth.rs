use super::NodeExt;
use axum::{
    body::Body,
    extract::FromRequest,
    http::{Method, Request, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
};
use cookie::Cookie;
use html_builder::prelude::*;
use serde::Deserialize;
use sha2::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Digest as _, Sha256,
};
use std::{future::Future, pin::Pin};
use tower_service::Service;

pub type Digest = GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize>;

#[derive(Clone, Debug)]
pub struct Auth<S> {
    inner: S,
    digest: Digest,
}

impl<S> Auth<S> {
    pub fn new(inner: S, digest: Digest) -> Self {
        Self { inner, digest }
    }
}

impl<S> Service<Request<Body>> for Auth<S>
where
    S: Service<Request<Body>, Response = Response> + Send + Clone + 'static,
    <S as Service<Request<Body>>>::Future: Send,
    <S as Service<Request<Body>>>::Error: Send,
{
    type Error = S::Error;
    type Response = Response;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let digest = self.digest;

        let cookies = req
            .headers()
            .get("cookie")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        for cookie in Cookie::split_parse_encoded(cookies).filter_map(Result::ok) {
            if cookie.name() == "auth" {
                let mut hasher = Sha256::new();
                hasher.update(cookie.value());
                let user_digest = hasher.finalize();

                if user_digest == digest {
                    let mut service = self.inner.clone();
                    return Box::pin(async move { service.call(req).await });
                }
            }
        }

        let fut = async move {
            let uri = req.uri().clone();

            if req.method() == Method::POST {
                #[derive(Deserialize, Debug)]
                struct Body {
                    password: String,
                    uri: String,
                }

                if let Ok(body) = axum::Form::<Body>::from_request(req, &()).await {
                    let password = &body.password;

                    let mut hasher = Sha256::new();
                    hasher.update(password);
                    let user_digest = hasher.finalize();

                    if user_digest == digest {
                        return Ok((
                            StatusCode::PERMANENT_REDIRECT,
                            AppendHeaders([
                                ("Set-Cookie", format!("auth={password}").as_str()),
                                ("Location", "/"),
                            ]),
                        )
                            .into_response());
                    }
                }
            }

            Ok((
                StatusCode::UNAUTHORIZED,
                Node::raw_document(
                    [form()
                        .class("flex items-center justify-center gap-4 flex-col h-full grow")
                        .attr("method", "post")
                        .child(input().attr("type", "text").attr("name", "password"))
                        .child(
                            input()
                                .attr("type", "hidden")
                                .attr("value", uri)
                                .attr("name", "uri"),
                        )
                        .child(input().attr("type", "submit").class("btn"))
                        .into()]
                    .into_iter(),
                ),
            )
                .into_response())
        };
        Box::pin(fut)
    }
}
