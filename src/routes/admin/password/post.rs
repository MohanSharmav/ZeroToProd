use crate::authentication::{self, AuthError, Credentials, UserId};
use crate::routes::admin::dashboard;
use crate::utils;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use validator::HasLen;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

impl FormData {
    pub fn new_password_length_check(&self) -> Result<(), AuthError> {
        match self.new_password.expose_secret().length() {
            13..=127 => Ok(()),
            _ => Err(anyhow::anyhow!(
                "The password must contain at least 13 and shorten then 128 chars."
            )
            .into()),
        }
    }
}

pub async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(utils::see_other("/admin/password"));
    }
    let username = dashboard::get_username(*user_id, &pool)
        .await
        .map_err(utils::e500)?;
    let credentials = Credentials {
        username,
        password: form.0.current_password.clone(),
    };
    if let Err(e) = form.new_password_length_check() {
        FlashMessage::error(e.to_string()).send();
        return Ok(utils::see_other("/admin/password"));
    }
    if let Err(e) = authentication::validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(utils::see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(utils::e500(e).into()),
        };
    }
    authentication::change_password(user_id, form.0.new_password, &pool)
        .await
        .map_err(utils::e500)?;
    FlashMessage::info("You password has been changed.").send();
    Ok(utils::see_other("/admin/password"))
}
