use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::services::otp_service::OTPService;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection,
};
use bcrypt::{hash, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::state::AppState;
use crate::models::user::User;
use crate::errors::{AppError, Result};

// Request DTOs
#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyOTPRequest {
    pub user_id: String,
    #[validate(length(min = 6, max = 6, message = "OTP must be 6 digits"))]
    pub otp: String,
    pub reset_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub user_id: String,
    pub reset_token: String,
    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub new_password: String,
}

// Response DTOs
#[derive(Debug, Serialize)]
pub struct ForgotPasswordResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<String>,
    pub reset_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerifyOTPResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub success: bool,
    pub message: String,
}

// 1. Forgot Password - Request OTP
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> impl IntoResponse {
    if let Err(errors) = req.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ForgotPasswordResponse {
                success: false,
                message: format!("Validation error: {}", errors),
                user_id: None,
                reset_token: None,
            })
        ).into_response();
    }

    let users: Collection<User> = state.db.collection("users");

    let user = match users.find_one(doc! { "username": &req.username }).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ForgotPasswordResponse {
                    success: false,
                    message: "User not found".to_string(),
                    user_id: None,
                    reset_token: None,
                })
            ).into_response();
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ForgotPasswordResponse {
                    success: false,
                    message: "Database error".to_string(),
                    user_id: None,
                    reset_token: None,
                })
            ).into_response();
        }
    };

    let user_id = match user._id {
        Some(id) => id,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ForgotPasswordResponse {
                    success: false,
                    message: "User ID not found".to_string(),
                    user_id: None,
                    reset_token: None,
                })
            ).into_response();
        }
    };

    let otp_code = OTPService::generate_otp();
    let reset_token = match state.otp_service.generate_reset_token(&user_id.to_hex()) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Token generation error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ForgotPasswordResponse {
                    success: false,
                    message: "Failed to generate reset token".to_string(),
                    user_id: None,
                    reset_token: None,
                })
            ).into_response();
        }
    };

    if let Err(e) = state.otp_service.store_otp_in_user(&user_id, &otp_code, &reset_token).await {
        tracing::error!("Failed to store OTP: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ForgotPasswordResponse {
                success: false,
                message: "Failed to store OTP".to_string(),
                user_id: None,
                reset_token: None,
            })
        ).into_response();
    }

    if let Err(e) = state.sms_service.send_otp(&user.phone, &otp_code).await {
        tracing::error!("Failed to send SMS: {}", e);
    }

    (
        StatusCode::OK,
        Json(ForgotPasswordResponse {
            success: true,
            message: "OTP sent to your phone".to_string(),
            user_id: Some(user_id.to_hex()),
            reset_token: Some(reset_token),
        })
    ).into_response()
}

// 2. Verify OTP
pub async fn verify_otp(
    State(state): State<AppState>,
    Json(req): Json<VerifyOTPRequest>,
) -> impl IntoResponse {
    let user_id = match ObjectId::parse_str(&req.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(VerifyOTPResponse {
                    success: false,
                    message: "Invalid user ID".to_string(),
                })
            ).into_response();
        }
    };

    match state.otp_service.verify_user_otp(&user_id, &req.otp, &req.reset_token).await {
        Ok(true) => {
            (
                StatusCode::OK,
                Json(VerifyOTPResponse {
                    success: true,
                    message: "OTP verified successfully".to_string(),
                })
            ).into_response()
        }
        Ok(false) => {
            (
                StatusCode::BAD_REQUEST,
                Json(VerifyOTPResponse {
                    success: false,
                    message: "Invalid or expired OTP".to_string(),
                })
            ).into_response()
        }
        Err(e) => {
            tracing::error!("OTP verification error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyOTPResponse {
                    success: false,
                    message: "Failed to verify OTP".to_string(),
                })
            ).into_response()
        }
    }
}

// 3. Reset Password
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> impl IntoResponse {
    let user_id = match ObjectId::parse_str(&req.user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ResetPasswordResponse {
                    success: false,
                    message: "Invalid user ID".to_string(),
                })
            ).into_response();
        }
    };

    let users: Collection<User> = state.db.collection("users");

    let hashed_password = match hash(&req.new_password, DEFAULT_COST) {
        Ok(pw) => pw,
        Err(e) => {
            tracing::error!("Password hashing error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResetPasswordResponse {
                    success: false,
                    message: "Failed to hash password".to_string(),
                })
            ).into_response();
        }
    };

    let now = chrono::Utc::now();
    let now_bson = mongodb::bson::DateTime::from_millis(now.timestamp_millis());

    let filter = doc! { "_id": user_id };
    let update = doc! {
        "$set": {
            "password_hash": hashed_password,
            "updated_at": now_bson
        },
        "$unset": { "reset_otp": "" }
    };

    match users.update_one(filter, update).await {
        Ok(result) if result.matched_count > 0 => {
            (
                StatusCode::OK,
                Json(ResetPasswordResponse {
                    success: true,
                    message: "Password reset successful".to_string(),
                })
            ).into_response()
        }
        Ok(_) => {
            (
                StatusCode::NOT_FOUND,
                Json(ResetPasswordResponse {
                    success: false,
                    message: "User not found".to_string(),
                })
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Password update error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResetPasswordResponse {
                    success: false,
                    message: "Failed to reset password".to_string(),
                })
            ).into_response()
        }
    }
}
