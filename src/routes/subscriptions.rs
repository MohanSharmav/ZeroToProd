use actix_web::{HttpResponse, web};
use sqlx::{ PgPool};
use tracing::Instrument;
use sqlx::types::uuid;
use chrono::Utc;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}

pub async fn subscribe(form: web::Form<FormData>,pool:web::Data<PgPool>,) -> HttpResponse{
    let request_id = Uuid::new_v4();//Uuid is used to generate a random id
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",//tracing when a new subscriber is added.
        %request_id,  //request_id of the subscriber to track the error better in the log trace
        subscriber_email = %form.email, //e-mail of the subscriber in the log trace
        subscriber_name = %form.name//name of the subscriber in the log trace

        // &a-->lj.23999909887,9.99898989,3.45888890,2.788889988=>{
        //     &b,&b,&v,&c(0.999,0.88,0.898,0.8876)
        //     #90099998.kjjj99999&88
        // }
    );
    //let_request_span_guard = request_span.enter();
    let query_span = tracing::info_span!("Saving new subscriber details in the database");
    match sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),//Uuid is used to generate random id for the user in the table
        form.email,//e-mail of the subscriber in the query
        form.name,//name of the subscriber in the query
        Utc::now()//timestamp when the query was created.
    )
        .execute(pool.get_ref())
        //First we attach the instrumentation, then we have to wait it out.
        .instrument(query_span)
        .await
    {
        Ok(_) => {
        //tracing::info!("request_id {} - New subscriber details have been saved",request_id);
        HttpResponse::Ok().finish()
        },
        Err(e) => {
            tracing::error!("request_id {} - Failed to execute query: {:?}",request_id,e);//log dependency is used to display errors.
            //println!("Failed to execute query: {}",e);
            HttpResponse::InternalServerError().finish()
        }
    }
}