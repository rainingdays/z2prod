use std::fmt::format;

use crate::domain::*;
use actix_web::web;
use actix_web::HttpResponse;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use sqlx::PgPool;
use tracing::subscriber::set_global_default;
use tracing::Instrument;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use uuid::Uuid;

// struct
#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
//     let request_id = Uuid::new_v4();
//     let request_span = tracing::info_span!(
//         "Adding a new subscriber.",
//         %request_id,
//         subscriber_email=%form.email,
//         subscriber_name=%form.name
//     );
//     let _request_span_guard = request_span.enter();
//     let query_span = tracing::info_span!("Saving new subscriber details in the database");

//     match sqlx::query!(
//         r#"Insert Into subscriptions (id, email, name, subscribed_at) Values ($1, $2, $3, $4)"#,
//         Uuid::new_v4(),
//         form.email,
//         form.name,
//         Utc::now()
//     )
//     .execute(pool.get_ref())
//     .instrument(query_span)
//     .await
//     {
//         Ok(_) => {
//             // tracing::info!(
//             //     "request_id {} - New subscriber details have been saved",
//             //     request_id
//             // );
//             HttpResponse::Ok().finish()
//         }
//         Err(e) => {
//             // println!("Failed to execute query:{}", e);
//             // tracing::error!(
//             //     "request_id {} - Failed to execute query: {:?}",
//             //     request_id,
//             //     e
//             // );
//             HttpResponse::InternalServerError().finish()
//         }
//     }
// }

#[tracing::instrument(
    name="Adding a new subscriber",
    skip(form, pool),
    fields(
        // request_id=%Uuid::new_v4(),
        subscriber_email=%form.email,
        subscriber_name=%form.name
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // let name = match SubscriberName::parse(form.0.name) {
    //     Ok(name) => name,
    //     Err(_) => return HttpResponse::BadRequest().finish(),
    // };
    // let email = match SubscriberEmail::parse(form.0.email) {
    //     Ok(email) => email,
    //     Err(_) => return HttpResponse::BadRequest().finish(),
    // };
    let new_subscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    // let new_subscriber = NewSubscriber { email, name };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        Insert Into subscriptions (id, email, name, subscribed_at) 
        Values ($1, $2, $3,$4)"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}
