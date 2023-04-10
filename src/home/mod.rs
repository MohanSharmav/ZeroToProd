mod login;
pub use login::*;
use actix_web::HttpResponse;
mod get;
pub use get::login_form;
pub async fn home()-> HttpResponse{
    HttpResponse:: Ok()
    .content_type(ContentType::html())
        .body(include_str!("home.html"))
}