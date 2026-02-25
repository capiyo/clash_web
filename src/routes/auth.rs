use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(crate::handlers::auth::register))
        .route("/login", post(crate::handlers::auth::login))
        .route("/login/phone", post(crate::handlers::auth::login_with_phone))
        .route("/users", get(crate::handlers::auth::get_all_users))
        .route("/profile/:id", get(crate::handlers::auth::get_user_profile))

}