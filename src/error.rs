use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("internal error: {0:?}")]
    Internal(Box<dyn std::error::Error>),

    #[error("error during initialization: {0:?}")]
    Init(Box<dyn std::error::Error>),

    #[error("not found")]
    NotFound,

    #[error("bad request, reason: {0:?}")]
    BadRequest(Option<String>),
}

impl ResponseError for AppError {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            AppError::NotFound => HttpResponse::NotFound().body("not found"),
            AppError::BadRequest(details) => {
                HttpResponse::BadRequest().body(details.clone().unwrap_or("bad request".to_owned()))
            }
            _ => HttpResponse::InternalServerError().body("Something went wrong"),
        }
    }
}
