use std::net::TcpListener;

use actix_web::cookie::time::format_description;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::Executor;
use sqlx::{Connection, PgConnection, PgPool};
use tracing::instrument::WithSubscriber;
use uuid::Uuid;
use z2prod::configuration::{self, get_configuration, DatabaseSettings};
use z2prod::email_client::EmailClient;
use z2prod::telemetry::*;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("test".into(), "trace".into(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

#[tokio::test]
async fn health_check_works() {
    // spawn_app().await.expect("Failed to spawn our app.");
    let app = spawn_app().await;
    let url = format!("{}/health_check", app.address);
    let client = reqwest::ClientBuilder::new().no_proxy().build().unwrap();
    // let client = reqwest::Client::new();
    let response = client
        // .get("http://127.0.0.1:8000/health_check")
        .get(&url)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    // assert_eq!(Some(0), response.content_length());
}

// async fn spawn_app() -> std::io::Result<()> {
//     z2prod::run().await
// }
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}
async fn spawn_app() -> TestApp {
    // let subscriber = get_subscriber("test".into(), "debug".into());
    // init_subscriber(subscriber);
    Lazy::force(&TRACING);
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    // let connection=PgConnection::connect()
    let address = format!("http://127.0.0.1:{}", port);
    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let timeout = configuration.email_client.timeout();
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    // let connection_pool = PgPool::connect(&configuration.database.connection_string())
    //     .await
    //     .expect("Failed to connect to database");
    let connection_pool = configure_database(&configuration.database).await;
    let server = z2prod::run(listener, connection_pool.clone(), email_client)
        .expect("Failed to bind address");
    let server_task_handle = tokio::spawn(server);
    // server_task_handle.is_finished();
    TestApp {
        address,
        db_pool: connection_pool,
    }
}
pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // let mut connection =
    //     PgConnection::connect(config.connection_string_without_db().expose_secret())
    //         .await
    //         .expect("Failed to connect to Postgres");
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"Create Database "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to logical database.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // let app_address = spawn_app();
    let app = spawn_app().await;
    // let configuration = get_configuration().expect("Failed to read configuration");
    // let configuration_string = configuration.database.connection_string();
    // dbg!(&configuration_string);
    // let mut connection = PgConnection::connect(&configuration_string)
    //     .await
    //     .expect("Failed to connect to Postgres.");
    let client = reqwest::ClientBuilder::new()
        .no_proxy()
        .build()
        .expect("client initialize failed");
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("Select email, name From subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // let app_address = spawn_app();
    let app = spawn_app().await;
    let client = reqwest::ClientBuilder::new()
        .no_proxy()
        .build()
        .expect("client initialize failed");
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("name=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let client = reqwest::ClientBuilder::new().no_proxy().build().unwrap();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];
    for (body, description) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}",
            description
        );
    }
}
