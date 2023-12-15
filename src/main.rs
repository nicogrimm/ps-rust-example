use std::env;

use actix_web::{web, App, HttpServer};
use deadpool_diesel::sqlite::{Manager, Pool, Runtime};

use ps_rust_example::*;

#[actix_web::main]
async fn main() {
    tracing_subscriber::fmt().json().init();
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

    match run().await {
        Ok(_) => {}
        Err(err) => {
            tracing::error!(?err, "An error occurred");
        }
    }
}

async fn run() -> Result<(), AppError> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, Runtime::Tokio1);
    let pool = Pool::builder(manager)
        .build()
        .map_err(|err| AppError::Init(Box::new(err)))?;

    tracing::info!("starting to listen on port 8080");
    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .app_data(web::Data::new(pool.clone()))
            .service(get_posts)
            .service(get_post_by_id)
            .service(create_post)
            .service(publish_post)
            .service(delete_post_by_text)
    })
    .bind(("127.0.0.1", 8080))
    .map_err(|err| AppError::Init(Box::new(err)))?
    .run()
    .await
    .map_err(|err| AppError::Init(Box::new(err)))
}
