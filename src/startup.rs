use crate::email_client::EmailClient;

// use super::routes::health_check;
use super::routes::*;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{dev::Server, web, App, HttpServer};
use env_logger::Env;
use env_logger::Logger as Logger_processor;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    // let connection = web::Data::new(connection);
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let db_pool = web::Data::new(db_pool);
    let email_client = Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            // .wrap(Logger::default())
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
