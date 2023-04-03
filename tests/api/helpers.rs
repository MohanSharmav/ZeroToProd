use ZeroToProd::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use ZeroToProd::configuration::{DatabaseSettings, get_configuration};
use ZeroToProd::email_client::EmailClient;
use ZeroToProd::startup::{Application, build, get_connection_pool, run};
use std::net::TcpListener;
use uuid::Uuid;
use wiremock::{Mock, MockServer};

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber("test".into(), "debug".into(),std::io::stdout);
    init_subscriber(subscriber);
});


pub struct ConfirmationLinks{
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp{
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
}

impl TestApp {

    pub fn get_confirmation_links(&self) ->email_request: &wiremock::Request ) -> ConfirmationLinks {
    let body: serde_json::Value = serde_json::from_slice( &email_request.body
    ).unwrap();
    let get_link = |s: &str| {
    let links: Vec<_> = linkify::LinkFinder::new()
    .links(s)
    .filter(|l| *l.kind() == linkify::LinkKind::Url) .collect();
    assert_eq!(links.len(), 1);
    let raw_link = links[0].as_str().to_owned();
    let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
    assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1"); confirmation_link.set_port(Some(self.port)).unwrap(); confirmation_link
    };
    let html = get_link(&body["HtmlBody"].as_str().unwrap());
    let plain_text = get_link(&body["TextBody"].as_str().unwrap());
    ConfirmationLinks {
    html,
    plain_text
    } }
}


    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address)) .header("Content-Type", "application/x-www-form-urlencoded") .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

pub async fn spawn_app() -> TestApp {

    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0") .expect("Failed to bind random port");

    let email_server=MockServer::start().await;

    let address= format!("http://127.0.0.1:{}",port);

    let configuration={
      let mut c=get_configuration().expect("Failed to get configuration");
        c.database.database_name=Uuid::new_v4().to_string();
        c.application.port=0;
        c.email_client.base_url= email_server.uri();
        c
    };

    let application= Application::build(
        configuration.clone())
            .await
            .expect("Failed to build application");

    let address= format!("http://127.0.0.1:{}",application.port());

let application_port=application.port();


    configure_database(&configuration.database).await;

    let connection_pool = configure_database(&configuration.database).await;

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");

    let timeout = configuration.email_client.timeout(); let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    let server= build(configuration.clone()).await.expect("Failed to build application");


    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
    }
}



pub async fn configure_database(config:&DatabaseSettings) -> PgPool{
    let mut connection = PgConnection::connect_with(
        &config.without_db()
    )
        .await
        .expect("Failed to connect Postgres");
    connection.execute(format!(r#"CREATE DATABASE"{}";"#,config.database_name).as_str())
        .await
        .expect("Failed to create database.");
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")//is the same macro used by sqlx-cli when executing sqlx migrate run - no need to throw bash scripts into the mix to achieve the same result
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool// returns the connection pool
}

