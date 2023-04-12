use crate::session_state::TypedSession;
use crate::utils;
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(utils::e500)?.is_none() {
        Ok(utils::see_other("/login"))
    } else {
        session.log_out();
        FlashMessage::info("You have successfully logged out.").send();
        Ok(utils::see_other("/login"))
    }
}
