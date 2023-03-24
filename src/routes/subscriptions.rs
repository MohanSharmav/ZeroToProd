use actix_web::{HttpResponse, web};
use sqlx::{ PgPool};
use tracing::Instrument;
use sqlx::types::uuid;
use chrono::Utc;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;
use crate::domain::{NewSubscriber, SubscriberName};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}



pub async fn subscribe(
    form: web::Form<FormData>,
    pool:web::Data<PgPool>,
) -> HttpResponse
{
    let name=match SubscriberName::parse(form.0.name)   {
        Ok(name)=>name,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let new_subscriber = NewSubscriber{
        email: form.0.email,
        name,
    };


//     let new_subscriber= NewSubscriber{
//     email: form.0.email,
//     name: SubscriberName::parse(form.0.name).expect("Name validation failed")
// };

   // let subscriber_name= crate::domain::SubscriberName(form.name.clone());
   //  let subscriber_name = SubscriberName(form.name.clone());
   //
   //  if !is_valid_name(&form.name) {
   //      return HttpResponse::BadRequest().finish()
   //  }

    match insert_subscriber(&pool, &new_subscriber).await{
        Ok(_)=> HttpResponse::Ok().finish(),
        Err(_)=> HttpResponse::InternalServerError().finish(),
    };


    let request_id = Uuid::new_v4();//Uuid is used to generate a random id
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",//tracing when a new subscriber is added.
        %request_id,  //request_id of the subscriber to track the error better in the log trace
        subscriber_email = %form.email, //e-mail of the subscriber in the log trace
        subscriber_name = %form.name//name of the subscriber in the log trace

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


pub fn is_valid_name(s: &str) -> bool{
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
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
)-> Result<(),sqlx::Error>{
    sqlx::query!(
        r#"
    INSERT INTO subscriptions(id,email,name,subscribed_at)
    values($1,$2,$3,$4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email,
        new_subscriber.name.as_ref(),
        Utc::now()
    )
        .execute(pool)
        .await
        .map_err(|e|{
            tracing::error!("failed to execute query:{:?}",e);
            e
        })?;
    Ok(())
}

