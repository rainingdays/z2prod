use reqwest::Url;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    // let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
    //     .await
    //     .unwrap();
    let response = app
        .http_client
        .get(&format!("{}/subscriptions/confirm", app.address))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    // reqwest::get(confirmation_links.html)
    //     .await
    //     .unwrap()
    //     .error_for_status()
    //     .unwrap();
    app.http_client
        .get(confirmation_links.html)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("Select email,name,status From subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path(""))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    // let get_link = |s: &str| {
    //     let links: Vec<_> = linkify::LinkFinder::new()
    //         .links(s)
    //         .filter(|l| *l.kind() == linkify::LinkKind::Url)
    //         .collect();

    //     assert_eq!(links.len(), 1);
    //     links[0].as_str().to_owned()
    // };

    // let raw_confirmation_link = &get_link(body["HtmlBody"].as_str().unwrap());
    // let mut confirmation_link = Url::parse(raw_confirmation_link).unwrap();
    // confirmation_link.set_port(Some(app.port)).unwrap();
    let confirmation_links = app.get_confirmation_links(email_request);

    // dbg!(&confirmation_link.as_str());

    let response = app
        .http_client
        .get(confirmation_links.html)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
}
