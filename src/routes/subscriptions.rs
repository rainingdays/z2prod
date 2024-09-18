use std::fmt::format;
use std::sync::mpsc::RecvTimeoutError;

use crate::domain::*;
use crate::email_client::EmailClient;
use crate::ApplicationBaseUrl;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use sqlx::PgPool;
use sqlx::{Postgres, Transaction};
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
    skip(form, pool, email_client, base_url),
    fields(
        // request_id=%Uuid::new_v4(),
        subscriber_email=%form.email,
        subscriber_name=%form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> HttpResponse {
    // let name = match SubscriberName::parse(form.0.name) {
    //     Ok(name) => name,
    //     Err(_) => return HttpResponse::BadRequest().finish(),
    // };
    // let email = match SubscriberEmail::parse(form.0.email) {
    //     Ok(email) => email,
    //     Err(_) => return HttpResponse::BadRequest().finish(),
    // };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let confirmation_link = "https://my-api.com/subscriptions/confirm";
    let new_subscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    // let new_subscriber = NewSubscriber { email, name };
    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscriber_token = generate_subscription_token();
    // if email_client
    //     .send_email(
    //         new_subscriber.email,
    //         "Welcome",
    //         // "Welcome to our newsletter!",
    //         &format!(
    //             "Welcome to our newsletter!<br />\
    //         Click <a href=\"{}\">here</a> to confirm your subscriptions.",
    //             confirmation_link
    //         ),
    //         &format!(
    //             "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
    //             confirmation_link
    //         ),
    //     )
    //     .await
    //     .is_err()
    // {
    //     return HttpResponse::InternalServerError().finish();
    // }
    if store_token(&mut transaction, subscriber_id, &subscriber_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscriber_token,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
}
#[derive(Debug)]
pub struct StoreTokenError(sqlx::Error);
impl ResponseError for StoreTokenError {}
impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to stiore a subscription token."
        )
    }
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    // pool: &PgPool,
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"Insert Into subscription_tokens(subscription_token, subscriber_id) 
        Values ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}
#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    // let confirmation_link = "https://my-api.com/subscriptions/confirm";
    let confirmation_link = format!(
        "http://{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    // pool: &PgPool,
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        Insert Into subscriptions (id, email, name, subscribed_at, status) 
        Values ($1, $2, $3,$4, 'pending_confirmation')"#,
        // Uuid::new_v4(),
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

// #[derive(serde::Deserialize)]
// pub struct Parameters {
//     subscription_token: String,
// }

// #[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
// pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
//     let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
//         Ok(id) => id,
//         Err(_) => return HttpResponse::InternalServerError().finish(),
//     };

//     match id {
//         None => HttpResponse::Unauthorized().finish(),
//         Some(subscriber_id) => {
//             if confirm_subscriber(&pool, subscriber_id).await.is_err() {
//                 return HttpResponse::InternalServerError().finish();
//             }
//             HttpResponse::Ok().finish()
//         }
//     }
// }

// #[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
// pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
//     sqlx::query!(
//         r#"Update subscriptions Set status='confirmed' Where id=$1"#,
//         subscriber_id
//     )
//     .execute(pool)
//     .await
//     .map_err(|e| {
//         tracing::error!("Failed to execute query:{:?}", e);
//         e
//     })?;
//     Ok(())
// }

// #[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
// pub async fn get_subscriber_id_from_token(
//     pool: &PgPool,
//     subscription_token: &str,
// ) -> Result<Option<Uuid>, sqlx::Error> {
//     let result = sqlx::query!(
//         r#"Select subscriber_id From subscription_tokens Where subscription_token =$1 "#,
//         subscription_token,
//     )
//     .fetch_optional(pool)
//     .await
//     .map_err(|e| {
//         tracing::error!("Failed to execute query:{:?}", e);
//         e
//     })?;
//     Ok(result.map(|r| r.subscriber_id))
// }

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
