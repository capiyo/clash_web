use mongodb::{
    bson::{doc, DateTime, oid::ObjectId},
    Collection, Database,
};
use rand::Rng;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};

use crate::models::user::User;
use crate::models::otp::ResetOTP;  // ADD THIS IMPORT
use crate::errors::{AppError, Result};

#[derive(Debug, Serialize, Deserialize)]
struct ResetClaims {
    user_id: String,
    purpose: String,
    exp: usize,
}

#[derive(Clone)]
pub struct OTPService {
    db: Database,
    jwt_secret: String,
}

impl OTPService {
    pub fn new(db: Database, jwt_secret: String) -> Self {
        Self { db, jwt_secret }
    }

    // Generate 6-digit OTP
    pub fn generate_otp() -> String {
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(0..1_000_000))
    }

    // Generate reset token
    pub fn generate_reset_token(&self, user_id: &str) -> Result<String> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::minutes(10))
            .ok_or_else(|| AppError::internal_server_error("Failed to calculate expiration"))?
            .timestamp() as usize;

        let claims = ResetClaims {
            user_id: user_id.to_string(),
            purpose: "password_reset".to_string(),
            exp: expiration,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        ).map_err(|e| AppError::internal_server_error(format!("Token generation failed: {}", e)))
    }

    // Store OTP in user document - FIXED VERSION
    pub async fn store_otp_in_user(
        &self,
        user_id: &ObjectId,
        code: &str,
        token: &str,
    ) -> Result<()> {
        let users: Collection<User> = self.db.collection("users");

        let now = Utc::now();
        let expires_at = now + Duration::minutes(5);

        // Create ResetOTP struct directly instead of using json!
        let reset_otp = ResetOTP {
            code: code.to_string(),
            token: token.to_string(),
            attempts: 0,
            expires_at: DateTime::from_millis(expires_at.timestamp_millis()),
            created_at: DateTime::from_millis(now.timestamp_millis()),
        };

        let filter = doc! { "_id": user_id };
        let update = doc! {
            "$set": {
                "reset_otp": bson::to_bson(&reset_otp).map_err(|e|
                    AppError::internal_server_error(format!("BSON conversion failed: {}", e))
                )?,
                "updated_at": DateTime::from_millis(now.timestamp_millis()),
            }
        };

        users.update_one(filter, update).await
            .map_err(|e| AppError::MongoDB(e))?;

        Ok(())
    }

    // Verify OTP from user document - FIXED VERSION
    pub async fn verify_user_otp(
        &self,
        user_id: &ObjectId,
        code: &str,
        token: &str,
    ) -> Result<bool> {
        let users: Collection<User> = self.db.collection("users");

        let user = users.find_one(doc! { "_id": user_id }).await
            .map_err(|e| AppError::MongoDB(e))?;

        if let Some(user) = user {
            let now = Utc::now();
            let now_millis = now.timestamp_millis();
            let now_bson = DateTime::from_millis(now_millis);

            // Check if user has reset_otp field
            if let Some(reset_otp) = user.reset_otp {
                // Check if valid - use timestamp_millis() for comparison
                let expires_at_millis = reset_otp.expires_at.timestamp_millis();

                if reset_otp.code == code
                    && reset_otp.token == token
                    && reset_otp.attempts < 3
                    && expires_at_millis > now_millis {

                    // Clear OTP after successful verification
                    let filter = doc! { "_id": user_id };
                    let update = doc! {
                        "$unset": { "reset_otp": "" },
                        "$set": { "updated_at": now_bson }
                    };

                    users.update_one(filter, update).await
                        .map_err(|e| AppError::MongoDB(e))?;

                    Ok(true)
                } else {
                    // Increment failed attempts
                    let filter = doc! { "_id": user_id };
                    let update = doc! {
                        "$inc": { "reset_otp.attempts": 1 },
                        "$set": { "updated_at": now_bson }
                    };

                    users.update_one(filter, update).await
                        .map_err(|e| AppError::MongoDB(e))?;

                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}
