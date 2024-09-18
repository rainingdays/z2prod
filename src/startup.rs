use crate::email_client::EmailClient;

// use super::routes::health_check;
use super::routes::*;
use crate::configuration::{self, *};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{dev::Server, web, App, HttpServer};
use env_logger::Env;
use env_logger::Logger as Logger_processor;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct ApplicationBaseUrl(pub String);

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    // let connection = web::Data::new(connection);
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let db_pool = web::Data::new(db_pool);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            // .wrap(Logger::default())
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/login", web::get().to(login_form))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route(
                "/subscriptions/confirm",
                web::get().to(crate::routes::subscriptions_confirm::confirm),
            )
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

// pub async fn build(configuration: Settings) -> Result<Server, std::io::Error> {
//     // let connection_pool = PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());

//     let connection_pool = get_connection_pool(&configuration.database);
//     let timeout = configuration.email_client.timeout();

//     let sender_email = configuration
//         .email_client
//         .sender()
//         .expect("Invalid sender email address.");
//     let email_client = EmailClient::new(
//         configuration.email_client.base_url,
//         sender_email,
//         configuration.email_client.authorization_token,
//         timeout,
//     );

//     let address = format!(
//         "{}:{}",
//         configuration.application.host, configuration.application.port
//     );
//     let listener = TcpListener::bind(address)?;
//     run(listener, connection_pool, email_client)
// }
pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = configuration.email_client.timeout();

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
        )?;

        Ok(Self { port, server })
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
