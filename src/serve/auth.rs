use super::Response;
use askama::Template;
use sha2::digest::generic_array::GenericArray;
use sha2::digest::OutputSizeUser;
use sha2::{Digest as _, Sha256};

pub type Digest = GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize>;

#[derive(Debug)]
pub struct LoginBody<'r> {
    password: &'r str,
    uri: &'r str,
}

pub struct Authed;

// #[rocket::async_trait]
// impl<'r> FromRequest<'r> for Authed {
//     type Error = ();

//     async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
//         let Outcome::Success(digest) = request.guard::<&State<Digest>>().await else {
//             return Outcome::Failure((Status::Unauthorized, ()));
//         };

//         let Some(cookie) = request.cookies().get("auth") else {
//             return Outcome::Failure((Status::Unauthorized, ()));
//         };

//         let mut hasher = Sha256::new();
//         hasher.update(cookie.value());
//         let user_digest = hasher.finalize();

//         if user_digest == **digest {
//             Outcome::Success(Self)
//         } else {
//             return Outcome::Failure((Status::Unauthorized, ()));
//         }
//     }
// }

// #[derive(Debug)]
// pub enum Login {
//     Success { password: String, location: String },
//     Failure,
// }

// #[rocket::async_trait]
// impl<'r> Responder<'r, 'static> for Login {
//     fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
//         match self {
//             Self::Success { password, location } => {
//                 let response = rocket::Response::build()
//                     .status(Status::MovedPermanently)
//                     .raw_header("Set-Cookie", format!("auth={password}; Http-Only; Secure"))
//                     .raw_header("Location", location)
//                     .finalize();

//                 Ok(response)
//             }
//             Self::Failure => Err(Status::Unauthorized),
//         }
//     }
// }

// #[post("/login", data = "<body>")]
// pub fn login(body: Form<LoginBody<'_>>, digest: &State<Digest>) -> Login {
//     let mut hasher = Sha256::new();
//     hasher.update(body.password);
//     let user_digest = hasher.finalize();

//     if user_digest == **digest {
//         Login::Success {
//             password: body.password.to_string(),
//             location: body.uri.to_string(),
//         }
//     } else {
//         Login::Failure
//     }
// }

// #[derive(Template)]
// #[template(path = "login.html")]
// struct LoginForm<'a> {
//     uri: Uri<'a>,
// }

// #[catch(401)]
// pub fn catch_unauthorized<'a>(req: &'a Request) -> Response<impl Template + 'a> {
//     let uri = (*req.uri()).clone().into();
//     Response::Partial(Status::Unauthorized, LoginForm { uri })
// }
