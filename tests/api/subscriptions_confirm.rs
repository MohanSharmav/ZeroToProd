use actix_web::{HttpResponse, web};
use reqwest::Url;
use sqlx::PgPool;
use tracing_subscriber::fmt::layer;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use crate::helpers::spawn_app;
#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
// Arrange
    let app = spawn_app().await;
// Act
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address)) .await
        .unwrap();
// Assert
    assert_eq!(response.status().as_u16(), 400);
}



#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
// Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email")).and(method("POST")).respond_with(ResponseTemplate::new(200)).mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);
// Act
    let response = reqwest::get(confirmation_links.html).await
        .unwrap();
// Assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber(){
    let app= spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;


    app.post_subscription(body.into()).await;
    let email_request= &app.email_server.received_requests().await.unwrap()[0];

    let confirmation_links= app.get_confirmation_links(&email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",) .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}


#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String
}
#[tracing::instrument(
name = "Confirm a pending subscriber", skip(parameters, pool)
)]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>, ) -> HttpResponse {
    let id = match get_subscriber_id_from_token(
        &pool,
        &parameters.subscription_token
    ).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(), };
    match id {
// Non-existing token!
        None => HttpResponse::Unauthorized().finish(), Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() { return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish() }
    }
}


#[tracing::instrument(
name = "Mark subscriber as confirmed", skip(subscriber_id, pool)
)]
pub async fn confirm_subscriber(
    pool: &PgPool,
    subscriber_id: Uuid
) -> Result<(), sqlx::Error> {
    sqlx::query!(
r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#, subscriber_id,
)        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e })?;
    Ok(())
}

#[tracing::instrument(
name = "Get subscriber_id from token", skip(subscription_token, pool)
)]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1", subscription_token,
    )
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e })?;
    Ok(result.map(|r| r.subscriber_id))
}



