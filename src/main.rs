use z2prod::configuration::*;
use z2prod::startup::*;
use z2prod::telemetry::*;
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

// #[tokio::main]
// async fn main() -> std::io::Result<()> {
//     let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
//     init_subscriber(subscriber);

//     let configuration = get_configuration().expect("Failed to read configuration.");
//     let address = format!(
//         "{}:{}",
//         configuration.application.host, configuration.application.port
//     );
//     // let connection = PgConnection::connect(&configuration.database.connection_string())
//     // .await
//     // .expect("Failed to connect to Postgres.");

//     // run("127.0.0.1:8000")?.await
//     // let listener = TcpListener::bind("127.0.0.1:8000").expect("failed to bind 8000 port");
//     let connection_pool =
//         // PgPool::connect_lazy(configuration.database.connection_string().expose_secret())
//         PgPoolOptions::new()
//         .connect_lazy_with(configuration.database.with_db());
//     // .await
//     // .expect("Failed to connect to Postgres.");
//     // let address = format!("127.0.0.1:{}", configuration.application_port);
//     let sender_email = configuration
//         .email_client
//         .sender()
//         .expect("Invalid sender email address.");
//     let timeout = configuration.email_client.timeout();
//     let email_client = EmailClient::new(
//         configuration.email_client.base_url,
//         sender_email,
//         configuration.email_client.authorization_token,
//         timeout,
//     );
//     let listener = TcpListener::bind(address)?;

//     run(listener, connection_pool, email_client)?.await
// }

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("z2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");
    // // let server = build(configuration).await?;
    // server.await?;
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
