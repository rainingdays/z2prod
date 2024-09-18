use actix_web::http::header::LOCATION;
use actix_web::web;
use actix_web::HttpResponse;
use secrecy::Secret;

//  pub async fn login() -> HttpResponse {
//     HttpResponse::SeeOther()
//         .insert_header((LOCATION, "/"))
//         .finish()
// }

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

pub async fn login(_form: web::Form<FormData>) -> HttpResponse {
    unimplemented!()
}
