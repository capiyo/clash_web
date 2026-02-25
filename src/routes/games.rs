use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;
use crate::handlers::games;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(games::get_games))
        .route("/stats", get(games::get_game_stats))
        .route("/recent", get(games::get_recent_games))
        .route("/:id", get(games::get_game_by_id))

}