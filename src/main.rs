use actix_web::{web, App, HttpRequest, HttpServer, Responder, HttpResponse};
//use ZeroToProd::run;
use tracing_log::LogTracer;
use ZeroToProd::startup::run;
use sqlx::postgres::PgPool;
use std::net::TcpListener;
//use env_logger::Env;
use ZeroToProd::telemetry::{get_subscriber, init_subscriber};
use tracing::Subscriber;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use ZeroToProd::configuration::get_configuration;

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    //env_logger::init();
    let subscriber = get_subscriber("ZeroToProd".into(),"info".into());
    init_subscriber(subscriber);
    //LogTracer::init().expect("Failed to set logger");
    // let env_filter = EnvFilter::try_from_default_env()
    //     .unwrap_or(EnvFilter::new("info"));
    // let formatting_layer = BunyanFormattingLayer::new(
    //     "ZeroToProd".into(),
    //     std::io::stdout
    // );
    // let subscriber = Registry::default()
    //     .with(env_filter)
    //     .with(JsonStorageLayer)
    //     .with(formatting_layer);
    // set_global_default(subscriber).expect("Failed to set subscriber");
    //env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(
        &configuration.database.connection_string()//configuration
    )
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}",configuration.application_port);
    let listener = TcpListener::bind(address)?;//TcpListener
    //Ok(())
    //println!("in main");
    run(listener,connection_pool)?.await?;
    Ok(())//await, connection_pool
}

// pub fn get_subscriber(
//     name: String,
//     env_filter: String
// ) -> impl Subscriber + Send + Sync {
//     let env_filter = EnvFilter::try_form_default_env()
//         .unwrap_or_else(|_| EnvFilter::new(env_filter));
//     let formatting_layer = BunyanFormattingLayer::new(
//         name,
//         std::io::stdout
//     );
//     Registry::default()
//         .with(env_filter)
//         .with(JsonStorageLayer)
//         .with(formatting_layer)
// }
//
// pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync){
//     LogTracer::init().expect("Failed to set logger");
//     set_global_default(subscriber).expect("Failed to set subscriber");
// }
// #[tokio::main]
// async fn main() -> Result<(), std::io::Error> {
//     HttpServer::new(|| {
//         App::new()
//         .route("/", web::get().to(greet))
//         .route("/{name}", web::get().to(greet))
//
//     })
//     .bind("127.0.0.1:8000")?
//     .run()
//     .await
//}
// async fn health_check(req: HttpRequest) -> impl
// Responder {
//     HttpResponse::Ok()
// }

// #[tokio::main]
// async fn main() -> Result<(), std::io::Error> {
//     HttpServer::new(|| {
//         App::new()
//             .route("/health_check",
//                    web::get().to(health_check))
//     })
//         .bind("127.0.0.1:8000")?
//         .run()
//         .await
// }
//pub fn set_logger(logger: &'static dyn Log) -> Result<(), SetLoggerError>

