use std::thread::current;
use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};
use ZeroToProd::authentication::AuthError;
use crate::helpers::{ConfirmationLinks, spawn_app, TestApp};

async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let ap = spawn_app().await;
    create_unconfirmed_subscriber(&app).awwait;


    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;


    let newsletter_request_body = serde_json::json!({         "title": "Newsletter title",
         "content": {
             "text": "Newsletter body as plain text",
             "html": "<p>Newsletter body as HTML</p>",
         }
});
    let response = app.post_newsletters(newsletter_request_body).await;

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address)).json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);

}



async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks
{
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    let response = app.post_newsletters(newsletter_request_body).await;

    app.get_confirmation_links(&email_request)


}

async fn create_confirmation_subscriber(app: &TestApp){
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
.await
.unwrap()
.error_for_status()
.unwrap();
}


#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers()
{
    let app= spawn_app().await;
    create_confirmed_subscriber(app).await;

    Mock::given(path("/email"))
        .add(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;



    let newsletter_request_body = serde_json::json!({ "title": "Newsletter title",
"content": {
             "text": "Newsletter body as plain text",
             "html": "<p>Newsletter body as HTML</p>",
        }
});
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address)) .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 200);


}


#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
// Arrange
    let app = spawn_app().await; let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
} }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}), "missing content",
        ), ];
    for (invalid_body, error_message) in test_cases {
        let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address)) .json(&invalid_body)
        .send()
        .await
        .expect("Failed to execute request.");

        for(invalid_body, error_message) in test_cases {
            let response= app.post_newsletters(invalid_body).await;
        }
// Assert
        assert_eq!( 400,
                    response.status().as_u16(),
                    "The API did not fail with 400 Bad Request when the payload was {}.",
                    error_message
        ); }
}


#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
// Arrange
    let app = spawn_app().await;
    let response = reqwest::Client::new() .post(&format!("{}/newsletters", &app.address)) .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
})) .send()
        .await
        .expect("Failed to execute request.");
// Assert
    assert_eq!(401, response.status().as_u16());
    assert_eq!(r#"Basic realm="publish""#, response.headers()["WWW-Authenticate"]);
}


#[tracing::instrument(name="Validate credentials", skip(credentails,pool))]
async fn validate_credentials(credentials: Credentials,
pool: &PgPool
)-> Result<uuid::Uuid, PublishError> {
    let (user_id, expected_password_hash) = get_stored_credentials(
        &credentials.username,
        &pool
    )
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| {
            PublishError::AuthError(anyhow::anyhow!("unknown username"))
        })?;


    tokio::task::spawn_blocking(move || {
        tracking::info_span!("verify password hash").in_scope(|| {
            Argon2::default()
                .verify_password(
                    credentials.password.expose_secret().as_bytes(),
                    expected_password_hash
                )
        })
    }
    )
        .await
        .context("Failed to spawn blocking task.")
        .map_err(PublishError::UnexpectedError)?
        .context("Invalid password.")
        .map_err(PublishError::AuthError)?;


    let expected_password_hash = PasswordHash::new(
        &expected_password_hash.expose_secret()
    )
        .map_err(PublishError::UnexpectedError)?;


    let current_span = tracking::Span::current();

    tokio::task::spawn_blocking(move || {
        current_span.in_scope(|| {
            verify_password_hash()
        })
    })

    tacking::info_span("verify password hash")
        .in_scope(||
            {
                Argon2::default()
                    .verify_password(
                            credentials.password.expose_secret().as_bytes(),
                            &edxpected_password_hash
                        )

            })
        .context("Invalid password")
        .map_err(PublishError::AuthError)?;
    Ok(user_id)
}


#[tokio::test]
async fn invalid_password_is_rejected(){
    let app= spawn_app().await;
    let username= &app.test_user.username;

    let password = Uuid::new_v4().to_string(); assert_ne!(app.test_user.password, password);
    let response = reqwest::Client::new() .post(&format!("{}/newsletters", &app.address)) .basic_auth(username, Some(password)) .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {

                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");
// Assert
    assert_eq!(401, response.status().as_u16()); assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}