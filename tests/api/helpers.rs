use std::net::TcpListener;

use actix_web::cookie::time::format_description;
use once_cell::sync::Lazy;
use sha3::Digest;
use sqlx::Executor;
use sqlx::{Connection, PgConnection, PgPool};
use tracing::instrument::WithSubscriber;
use uuid::Uuid;
use wiremock::MockServer;
use z2prod::configuration::{self, get_configuration, DatabaseSettings};
use z2prod::email_client::EmailClient;
use z2prod::startup::*;
use z2prod::startup::*;
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

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    //a general no-proxy http client
    pub http_client: reqwest::Client,
    test_user: TestUser,
}
pub async fn spawn_app() -> TestApp {
    // let subscriber = get_subscriber("test".into(), "debug".into());
    // init_subscriber(subscriber);
    Lazy::force(&TRACING);
    // let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // let port = listener.local_addr().unwrap().port();
    // let connection=PgConnection::connect()
    // let address = format!("http://127.0.0.1:{}", port);
    let mut email_server = MockServer::start().await;
    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.application.port = 0;
    configuration.email_client.base_url = email_server.uri();
    configuration.database.database_name = Uuid::new_v4().into();
    let db_pool = configure_database(&configuration.database).await;

    // let server = build(configuration.clone())
    //     .await
    //     .expect("Failed to build application.");
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let address = format!("http://127.0.0.1:{}", application.port());
    let port = application.port();
    dbg!(&address);
    dbg!(&configuration.database.port);
    let _ = tokio::spawn(application.run_until_stopped());
    configuration.database.database_name = Uuid::new_v4().to_string();
    // let timeout = configuration.email_client.timeout();
    // let sender_email = configuration
    // .email_client
    // .sender()
    // .expect("Invalid sender email address.");
    // let email_client = EmailClient::new(
    // configuration.email_client.base_url,
    // sender_email,
    // configuration.email_client.authorization_token,
    // timeout,
    // );

    // let connection_pool = PgPool::connect(&configuration.database.connection_string())
    //     .await
    //     .expect("Failed to connect to database");
    // let connection_pool = configure_database(&configuration.database).await;
    // let server = z2prod::run(listener, connection_pool.clone(), email_client)
    // .expect("Failed to bind address");
    // let server_task_handle = tokio::spawn(server);
    // server_task_handle.is_finished();
    let http_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build a http client.");
    let test_app = TestApp {
        address,
        db_pool,
        email_server,
        port,
        http_client,
        test_user: TestUser::generate(),
    };
    // add_test_user(&test_app.db_pool).await;
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
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

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("failed to build a request client.");

        client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        //adapt to test env
        ConfirmationLinks { html, plain_text }
    }
    // pub async fn test_user(&self) -> (String, String) {
    //     let row = sqlx::query!("Select username, password_hash From users LIMIT 1",)
    //         .fetch_one(&self.db_pool)
    //         .await
    //         .expect("Failed to create test users.");

    //     (row.username, row.password_hash)
    // }
    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        // let (username, password) = self.test_user().await;
        self.http_client
            .post(&format!("{}/newsletters", &self.address))
            // .basic_auth(username, Some(password))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

// async fn add_test_user(pool: &PgPool) {
//     let password_hash = "testpass";
//     sqlx::query!(
//         "Insert Into users (user_id, username, password_hash) Values ($1, $2, $3)",
//         Uuid::new_v4(),
//         Uuid::new_v4().to_string(),
//         password_hash
//     )
//     .execute(pool)
//     .await
//     .expect("Failed to create test users.");
// }

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}
impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let password_hash = sha3::Sha3_256::digest(self.password.as_bytes());

        let password_hash = format!("{:x}", password_hash);
        sqlx::query!(
            "Insert Into users (user_id, username, password_hash) Values ($1,$2,$3)",
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}
