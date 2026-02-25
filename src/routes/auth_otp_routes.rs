use axum::{
    routing::post,
    Router,
};

use crate::{
    handlers::auth_otp,
    state::AppState,
};

pub fn auth_otp_routes() -> Router<AppState> {
    Router::new()
        // Request OTP for password reset
        .route("/auth/forgot-password", post(auth_otp::forgot_password))

        // Verify OTP
        .route("/auth/verify-otp", post(auth_otp::verify_otp))

        // Reset password with verified OTP
        .route("/auth/reset-password", post(auth_otp::reset_password))
}
