use std::error::Error;
use std::fmt::{Debug, Display, Formatter, write};
use actix_web::{HttpResponse, ResponseError, web};
use actix_web::http::StatusCode;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::Instrument;
use sqlx::types::uuid;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}

pub fn parse_subscriber(form: FormData) ->Result<NewSubscriber,String>
{
    let name= SubscriberName::parse(form.name)?;
    let email = SubscriberEmail::parse(form.email)?;
    Ok(NewSubscriber{email,name})
}
impl TryFrom<FormData> for NewSubscriber
{
    type Error = String;

    fn try_from(value: FormData) -> Result<Self,Self::Error>{
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self{email,name})
    }
}


#[tracing::instrument(
name = "Adding a new subscriber", skip(form, pool, email_client),
fields(
subscriber_email = %form.email,
subscriber_name = %form.name
)
)]

pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    base_url: web::Data<ApplicationBaseUrl>,
) ->Result<HttpResponse,SubscribeError>
{

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

//todo:
    let subscriber_id = insert_subscriber(/* */)
        .await
        .context("Failed to insert new subscriber in the database.")?;


    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;




    let subscription_token = generate_subscription_token();

    if store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;


    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;



    let new_subscriber = match form.0.try_into()
    {
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    if insert_subscriber(&pool, &new_subscriber).await.is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    transcation
        .commit()
        .await
        .map_err(SubscribeError::TransactionCommitError)?;


    if send_confirmation_email(&email_client,
                               new_subscriber,
    &base_url.0,
    my_token)
        .await
        .context("Failed to send a confirmation email.")?;


    HttpResponse::Ok().finish()
        .await
        .map_err(|e| SubscribeError::UnexpectedError(Box::new(e)))?;

    Ok(HttpResponse::Ok().finish());
}



#[tracing::instrument(
name = "Send a confirmation email to a new subscriber",
skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url,
        subscription_token
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.", confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &html_body,
            &plain_body,
        )
        .await
}



pub fn is_valid_name(s: &str) -> bool
{
    let is_empty_or_whitespace = s.trim().is_empty();

    let is_too_long= s.graphemes(true).count()>256;

    let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

    let contains_forbidden_characters = s
        .chars()
        .any(|g|forbidden_characters.contains(&g));


    !(is_empty_or_whitespace||is_too_long||contains_forbidden_characters)
}

#[tracing::instrument(
name="Saving new subscriber deatails in the database"
skip(new_subscriber,pool)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber, )
    -> Result<Uuid, sqlx::Error>
{
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
r#"INSERT INTO subscriptions
(id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
        .execute(transaction)
        .await
        .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e })?;
    Ok(())
}

impl SubscriberName {
    pub fn inner(self) -> String {
        // The caller gets the inner string,
        // but they do not have a SubscriberName anymore!
        // That's because `inner` takes `self` by value,
        // consuming it according to move semantics
        self.0
    }
}


fn generate_subscription_token() -> String
{
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}



#[tracing::instrument(
name = "Store subscription token in the database",
skip(subscription_token, pool)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e);
        e;
    })?;
    Ok(())
}

impl TryFrom<FormData> for NewSubscriber
{
type Error = String;

    fn try_from(value: FormData)->Result<Self, Self::Error>
    {
        let name= SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self{email,name})
    }
}



#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
// Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
// Sabotage the database
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;",)
        .execute(&app.db_pool)
        .await
        .unwrap();
    ;
// Act
    let response =  app.post_subscriptions(body.into()).await;
// Assert
    assert_eq!(response.status().as_u16(), 500);
}

#[derive(Debug)]
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\nCaused by:\n\t{}", self, self.0)
    }
}

impl std::fmt::Debug for StoreTokenError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
)-> std::fmt::Result
{
    writeln!(f, "{}\n", e)?;
    let mut current = e.source(); while let Some(cause) = current {
    writeln!(f, "Caused by:\n\t{}", cause)?;
    current = cause.source();
}
    Ok(())
}



impl std::fmt::Display for SubscribeError{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to create a new subscriber"
        )
    }
}

//
// #[derive(thiserror::Error)]
// pub enum SubscribeError {
//     #[error("{0}")]
//     ValidationError(String),
//     #[error("Failed to acquire a Postgres connection from the pool")] PoolError(#[source] sqlx::Error),
//     #[error("Failed to store the confirmation token for a new subscriber.")] StoreTokenError(#[from] StoreTokenError),
//     #[error("Failed to commit SQL transaction to store a new subscriber.")] TransactionCommitError(#[source] sqlx::Error),
//     #[error("Failed to send a confirmation email.")]
//     SendEmailError(#[from] reqwest::Error),
//     #[error(transparent)]
//     UnexpectedError(#[from] Box<dyn std::error::Error>),
// #[error("Failed to insert new subscriber in the database.")]InsertSubscriberError(#[source] sqlx::Error),
// }

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}


impl From<reqwest::Error> for SubscribeError { fn from(e: reqwest::Error) -> Self {
    Self::SendEmailError(e) }
}
impl From<sqlx::Error> for SubscribeError { fn from(e: sqlx::Error) -> Self {
    Self::DatabaseError(e) }
}
impl From<StoreTokenError> for SubscribeError {
    fn from(e: StoreTokenError) -> Self {
        Self::StoreTokenError(e) }
}
impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
    Self::ValidationError(e)
}
}
impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { error_chain_fmt(self, f)
    }
}


impl Debug for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

    }
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,

        }
    }
}

impl std::fmt::Debug for SubscribeError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SubscribeError::ValidationError(_) => None,
            SubscribeError::StoreTokenError(e) => Some(e),
            SubscribeError::SendEmailError(e) => Some(e),
            _ => {}
        }
        }
}



impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // [...]
            SubscribeError::PoolError(_) => {
                write!(f, "Failed to acquire a Postgres connection from the pool")
            }
            SubscribeError::InsertSubscriberError(_) => {
                write!(f, "Failed to insert new subscriber in the database.") }
            SubscribeError::TransactionCommitError(_) => { write!(
                f,
                "Failed to commit SQL transaction to store a new subscriber."
            )
            }
            _ => {}
        }
    }
}


impl From<sqlx::Error> for SubscribeError {
    fn from(e: sqlx::Error) -> Self {
        Self::DatabaseError(e)
    }
}