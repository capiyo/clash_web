use axum::{
    Router,
    routing::{get, post},
    Json,
};
use chrono::Utc;
use serde_json::json;

use crate::state::AppState;
use crate::handlers::mpesa_handlers;
use crate::handlers::b2c_handlers;

pub fn mpesa_routes() -> Router<AppState> {
    Router::new()
        // Health
        .route("/health", get(mpesa_health))

        // C2B Routes
        .route("/stk-push", post(mpesa_handlers::initiate_stk_push))
        .route("/callback", post(mpesa_handlers::mpesa_callback))

        // ✅ NEW: Payment status check endpoint (POST for frontend)
        .route("/check-payment-status", post(mpesa_handlers::check_payment_status))

        // B2C Routes
        .route("/b2c/send", post(b2c_handlers::send_b2c_payment))
        .route("/b2c/result", post(b2c_handlers::b2c_result_callback))
        .route("/b2c/timeout", post(b2c_handlers::b2c_timeout_callback))

        // Status (GET with query params)
        .route("/status", get(mpesa_handlers::check_transaction_status))
        .route("/transactions", get(mpesa_handlers::get_transactions))
        .route("/stats", get(mpesa_handlers::get_stats))
        .route("/simulate", post(mpesa_handlers::simulate_payment))
}

async fn mpesa_health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "mpesa",
        "timestamp": Utc::now().to_rfc3339(),
        "features": ["c2b", "b2c", "stk-push", "transactions", "payment-status-check"]
    }))
}