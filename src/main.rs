use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use sqlx::{postgres::PgConnectOptions, Connection, PgConnection};
use std::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};
use z2prod::{configuration, telemetry::*};
use z2prod::{configuration::get_configuration, run};

// async fn greet(req: HttpRequest) -> impl Responder {
//     let name = req.match_info().get("name").unwrap_or("World");
//     format!("Hello {}!", &name)
// }
// async fn health_check(req: HttpRequest) -> impl Responder {
//     HttpResponse::Ok().body("its health!")
// }

// #[tokio::main]
// async fn main() -> std::io::Result<()> {
//     LogTracer::init().expect("Failed to set logger");
//     let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
//     let formattin_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout);
//     let subscriber = Registry::default()
//         .with(env_filter)
//         .with(JsonStorageLayer)
//         .with(formattin_layer);

//     set_global_default(subscriber).expect("Failed to set subscriber");
//     let configuration = get_configuration().expect("Failed to read configuration.");
//     // let connection = PgConnection::connect(&configuration.database.connection_string())
//     // .await
//     // .expect("Failed to connect to Postgres.");

//     // run("127.0.0.1:8000")?.await
//     // let listener = TcpListener::bind("127.0.0.1:8000").expect("failed to bind 8000 port");
//     let connection_pool = PgPool::connect(&configuration.database.connection_string())
//         .await
//         .expect("Failed to connect to Postgres.");
//     let address = format!("127.0.0.1:{}", configuration.application_port);
//     let listener = TcpListener::bind(address)?;

//     run(listener, connection_pool)?.await
// }

// pub fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
//     let env_filter =
//         EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
//     let formatting_layer = BunyanFormattingLayer::new(name, std::io::stdout);
//     Registry::default()
//         .with(env_filter)
//         .with(JsonStorageLayer)
//         .with(formatting_layer)
// }

// pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
//     LogTracer::init().expect("Failed to set logger");
//     set_global_default(subscriber).expect("Failed to set subscriber");
// }

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    // let connection = PgConnection::connect(&configuration.database.connection_string())
    // .await
    // .expect("Failed to connect to Postgres.");

    // run("127.0.0.1:8000")?.await
    // let listener = TcpListener::bind("127.0.0.1:8000").expect("failed to bind 8000 port");
    let connection_pool =
        // PgPool::connect_lazy(configuration.database.connection_string().expose_secret())
        PgPoolOptions::new()
        .connect_lazy_with(configuration.database.with_db());
    // .await
    // .expect("Failed to connect to Postgres.");
    // let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}
