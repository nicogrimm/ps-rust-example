use actix_web::{delete, get, post, web};
use deadpool_diesel::sqlite::Pool;
use diesel::prelude::*;

pub use error::AppError;
use models::*;
use serde::Deserialize;

mod error;
pub mod models;
pub mod schema;

#[derive(Deserialize)]
pub struct GetPostsQueryParams {
    include_unpublished: Option<bool>,
}

#[get("/post")]
pub async fn get_posts(
    pool: web::Data<Pool>,
    query: web::Query<GetPostsQueryParams>,
) -> Result<web::Json<Vec<Post>>, AppError> {
    let posts = utils::do_query(pool, move |db_conn| {
        use schema::posts::dsl::*;

        if query.include_unpublished.unwrap_or(false) {
            posts.into_boxed()
        } else {
            posts.filter(published.eq(true)).into_boxed()
        }
        .limit(20)
        .select(Post::as_select())
        .load(db_conn)
    })
    .await?;

    Ok(web::Json(posts))
}

#[get("/post/{id}")]
pub async fn get_post_by_id(
    pool: web::Data<Pool>,
    path: web::Path<i32>,
) -> Result<web::Json<Post>, AppError> {
    let post_id = path.into_inner();

    let post = utils::do_query(pool, move |db_conn| {
        use schema::posts::dsl::*;

        posts
            .find(post_id)
            .select(Post::as_select())
            .first(db_conn)
            .optional()
    })
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(web::Json(post))
}

#[post("/post")]
pub async fn create_post(
    pool: web::Data<Pool>,
    post: web::Json<NewPost>,
) -> Result<web::Json<Post>, AppError> {
    let post = utils::do_query(pool, move |db_conn| {
        use schema::posts;

        diesel::insert_into(posts::table)
            .values(post.0)
            .returning(Post::as_returning())
            .get_result(db_conn)
    })
    .await?;

    Ok(web::Json(post))
}

#[post("/post/{id}/publish")]
pub async fn publish_post(
    pool: web::Data<Pool>,
    path: web::Path<i32>,
) -> Result<web::Json<Post>, AppError> {
    let post_id = path.into_inner();

    let post = utils::do_query(pool, move |db_conn| {
        use schema::posts::dsl::*;

        diesel::update(posts.find(post_id))
            .set(published.eq(true))
            .returning(Post::as_returning())
            .get_result(db_conn)
    })
    .await?;

    Ok(web::Json(post))
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    text: Option<String>,
    id: Option<i32>,
}

#[delete("/post")]
pub async fn delete_post_by_text(
    pool: web::Data<Pool>,
    query: web::Json<DeleteQuery>,
) -> Result<web::Json<usize>, AppError> {
    if [query.text.is_some(), query.id.is_some()]
        .iter()
        .filter(|b| **b)
        .count()
        != 1
    {
        return Err(AppError::BadRequest(Some(
            "request needs exactly one of 'text' or 'id'".to_owned(),
        )));
    }

    let num_deleted = utils::do_query(pool, move |db_conn| {
        use schema::posts::dsl::*;

        if let Some(text) = &query.text {
            let pattern = format!("%{}%", text);

            diesel::delete(posts.filter(title.like(pattern))).execute(db_conn)
        } else if let Some(post_id) = query.id {
            diesel::delete(posts.filter(id.eq(post_id))).execute(db_conn)
        } else {
            unreachable!()
        }
    })
    .await?;

    Ok(web::Json(num_deleted))
}

/// utilities to use within the lib.rs
mod utils {
    use std::error::Error;

    use actix_web::web;
    use deadpool_diesel::sqlite::Pool;
    use diesel::SqliteConnection;

    use crate::AppError;

    fn query_error(err: impl Error + 'static) -> AppError {
        tracing::error!(?err, "error querying database");
        AppError::Internal(Box::new(err))
    }

    /// do a query against the database
    pub async fn do_query<F, R>(pool: web::Data<Pool>, query: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<R, diesel::result::Error> + Send + 'static,
        R: Send + 'static,
    {
        let db_conn = pool.get().await.map_err(|err| {
            tracing::error!(?err, "failed to get a db connection from the pool");
            AppError::Internal(Box::new(err))
        })?;

        db_conn
            .interact(query)
            .await
            .map_err(query_error)?
            .map_err(query_error)
    }
}
