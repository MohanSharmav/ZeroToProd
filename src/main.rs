use actix_web::{web, App, HttpRequest, HttpServer, Responder, HttpResponse};
//use ZeroToProd::run;
use tracing_log::LogTracer;
use ZeroToProd::startup::{Application, run};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::net::TcpListener;
use tokio::time::timeout;
//use env_logger::Env;
use ZeroToProd::telemetry::{get_subscriber, init_subscriber};
use tracing::Subscriber;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use ZeroToProd::configuration::{get_configuration, Settings};
use ZeroToProd::email_client::EmailClient;
use zero2prod::startup::Application;
//
async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error>
{

    let subscriber = get_subscriber(
        "ZeroToProd".into(), "info".into(), std::io::stdout
    );
    init_subscriber(subscriber);


    let configuration = get_configuration().expect("Failed to read configuration.");

    let server = build(configuration).await?;
    server.await?;



    let connection_pool= PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    let sender_email= configuration
        .email_client.sender()
        .expect("invalid sender email address");

    let timeout = configuration.email_client.timeout();

    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout
    );

    let address = format!(
        "{}:{}",
        configuration.application.host,configuration.application.port);
    let listener = TcpListener::bind(address)?;//TcpListener

    let application= Application::build(configuration).await?;
    application.run_until_stopped().await?;

    run(listener,connection_pool,email_client)?.await?;
    Ok(())


}


