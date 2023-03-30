use std::time::Duration;
use fake::{Fake, Faker};
use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::raw::{Paragraph, Sentence};
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use tracing_subscriber::fmt::time;
use wiremock::matchers::{header, header_exists, method};
use wiremock::{Mock, MockServer};
use crate::domain::SubscriberEmail;


#[derive(serde::Serialize)]
#[serde(rename_all="PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}



pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>
}


impl EmailClient
{

    pub async fn send_email(
        &self,
    recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str
    )-> Result<(),reqwest::Error>
    {
        let url = format!("{}/email", self.base_url);


        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        let builder = self
            .http_client
            .post(&url).json(&request_body)
            .header("X-Postmark-Server-Token", self.authorization_token.expose_secret()
            )
            .json(&request_body)
            .send()
            .await?;
        Ok(())

//todo

        self.http_client
            .post(&url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret()
            )
            .json(&request_body)
            .send()
            .await?;
        Ok(())
    }


    pub fn new(base_url: String,
               sender: SubscriberEmail,
    timeout: std::time::Duration) ->Self
    {

           let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();

            Self{
            http_client,
            base_url,
            sender,
            authorization_token
            }

        }
    }



#[cfg(test)]
mod tests {
    use actix_web::middleware::ErrorHandlerResponse::Response;
    use claims::assert_ok;
    use config::ValueKind::String;
    use fake::{Fake, Faker};
    use fake::faker::internet::raw::SafeEmail;
    use fake::faker::lorem::raw::{Paragraph, Sentence};
    use secrecy::Secret;
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};
    use wiremock::matchers::{any, header, header_exists, method, path};
    use crate::domain::SubscriberEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use crate::email_client::EmailClient;
    use claims::assert_err;

    struct SendEmailBodyMatcher;


    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
// Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap(); let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake()) );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap(); let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        let response = ResponseTemplate::new(200) // 3 minutes!
            .set_delay(Duration::from_secs(180)); Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
// Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
        // Assert
        assert_err!(outcome); }
}


    impl wiremock::Match for SendEmailBodyMatcher
    {
        fn matches(&self, request: &Request) -> bool
        {
            let result: Result<serde_json::Value,_>=
            serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                dbg!(&body);
                body.get("From").is_some()
                && body.get("To").is_some()
                && body.get("Subject").is_some()
                && body.get("HtmlBody").is_some()
                && body.get("TextBody").is_some()
            }else{
                false
            }

        }


    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url()
    {
        let mock_server= MockServer::start().await;
        let sender= SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = SubscriberEmail::new
            (mock_server.uri(),
             sender,
            Secret::new(Faker.fake())
            );

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1 ..2).fake();
        let content: String = Paragraph(1 .. 10).fake();

        let _=email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200()
    {
        let mock_server= MockServer::start.await;
        let sender= SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client =EmailClient::new (
          mock_server.uri(),
            sender,
            Secret::new(Faker.fake())
        );

        let subscriber_email= SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();


        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mock_(&mock_server)
            .await;


        let outcome= email_client
            .send_email(subscriber_email,&subject,&content,&content)
            .await;

        assert_ok!(outcome)


    }
    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
// Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap(); let email_client = EmailClient::new(
            mock_server.uri(), sender, Secret::new(Faker.fake())
        );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap(); let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        Mock::given(any())
// Not a 200 anymore! .respond_with(ResponseTemplate::new(500)) .expect(1)
            .mount(&mock_server)
            .await;
// Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
        // Assert
        assert_err!(outcome); }


fn subject()->String {
    Sentence(1..2).fake()
}
fn content()->String {
    Paragraph(1..10).fake()
}


/// Generate a random subscriber email
fn email() -> SubscriberEmail
{ SubscriberEmail::parse(SafeEmail().fake()).unwrap()
}
/// Get a test instance of `EmailClient`.
fn email_client(base_url: String) -> EmailClient {
    EmailClient::new(
        base_url,
        email(), Secret::new(Faker.fake()),
        Duration::from_millis(200),
    )

}


#[tokio::test]
async fn send_email_sends_the_expected_request() {
// Arrange
    let mock_server = MockServer::start().await;
    let email_client = email_client(mock_server.uri());
    Mock::given(header_exists("X-Postmark-Server-Token")).and(header("Content-Type", "application/json")).and(path("/email"))
        .and(method("POST"))
        .and(SendEmailBodyMatcher).respond_with(ResponseTemplate::new(200)).expect(1)
        .mount(&mock_server)
        .await;
// Act
    let _ = email_client
        .send_email(email(), &subject(), &content(), &content())
        .await;
}



