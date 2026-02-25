use reqwest::Client;
use serde_json::{json, Value};
use mongodb::{Collection, bson::{doc, DateTime as BsonDateTime}};
use yup_oauth2::ServiceAccountKey;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::TryStreamExt;
use std::env;
use std::time::Instant;
use yup_oauth2::parse_service_account_key;

use crate::{
    errors::AppError,
    models::notification::{FCMToken, Notification},
    state::AppState,
};

const FIREBASE_PROJECT_ID: &str = "clash-66865";

pub struct FCMService {
    authenticator: Arc<Mutex<yup_oauth2::authenticator::Authenticator<
        yup_oauth2::hyper_rustls::HttpsConnector<hyper::client::HttpConnector>
    >>>,
    client: Client,
}

impl FCMService {
pub async fn new() -> anyhow::Result<Self> {
    println!("\n🔧🔧🔧 FCM SERVICE INITIALIZATION STARTED 🔧🔧🔧");
    println!("📖 Reading Firebase credentials from environment variables...");

    // Get credentials from .env
    let client_email = env::var("FIREBASE_CLIENT_EMAIL")
        .map_err(|_| anyhow!("FIREBASE_CLIENT_EMAIL not set in environment"))?;

    let private_key = env::var("FIREBASE_PRIVATE_KEY")
        .map_err(|_| anyhow!("FIREBASE_PRIVATE_KEY not set in environment"))?;

    let project_id = env::var("FIREBASE_PROJECT_ID")
        .unwrap_or_else(|_| "clash-66865".to_string());

    println!("✅ FIREBASE_CLIENT_EMAIL found: {}", client_email);
    println!("✅ FIREBASE_PRIVATE_KEY found (length: {} chars)", private_key.len());
    println!("   Preview: {}...", &private_key[0..50.min(private_key.len())]);
    println!("✅ Using project_id: {}", project_id);

    // Create a JSON string from the environment variables
    println!("🔨 Creating service account JSON...");

    // Clean the private key - ensure it has proper line breaks
    let cleaned_private_key = private_key.replace("\n", "\\n");

    let service_account_json = format!(r#"{{
  "type": "service_account",
  "project_id": "{}",
  "private_key_id": "",
  "private_key": "{}",
  "client_email": "{}",
  "client_id": "",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token",
  "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
  "client_x509_cert_url": "https://www.googleapis.com/robot/v1/metadata/x509/{}"
}}"#, project_id, cleaned_private_key, client_email, client_email.replace("@", "%40"));

    println!("✅ Service account JSON created ({} bytes)", service_account_json.len());

    // Parse it using the library's function
    println!("🔨 Parsing service account key...");
    let service_account_key = parse_service_account_key(service_account_json.as_bytes())
        .map_err(|e| anyhow!("Failed to parse service account key: {}", e))?;

    println!("✅ Service account key parsed successfully");

    // Build the authenticator
    println!("🔨 Building authenticator...");
    let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .map_err(|e| anyhow!("Failed to build authenticator: {}", e))?;

    println!("✅ Authenticator built successfully");
    println!("🔧🔧🔧 FCM SERVICE INITIALIZATION COMPLETE 🔧🔧🔧\n");

    Ok(Self {
        authenticator: Arc::new(Mutex::new(authenticator)),
        client: Client::new(),
    })
}

    pub  async fn get_access_token(&self) -> anyhow::Result<String> {
        println!("🔑 [FCM] Requesting access token from Google...");
        let start = Instant::now();

        let auth = self.authenticator.lock().await;
        println!("🔑 [FCM] Auth lock acquired in {:?}", start.elapsed());

        let token_result = auth
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await;

        match token_result {
            Ok(token) => {
                println!("✅ [FCM] Token received in {:?}", start.elapsed());
                match token.token() {
                    Some(token_str) => {
                        println!("✅ [FCM] Token string extracted, length: {}", token_str.len());
                        println!("✅ [FCM] Token preview: {}...", &token_str[0..20.min(token_str.len())]);
                        Ok(token_str.to_string())
                    }
                    None => {
                        println!("❌ [FCM] Token object existed but token() returned None");
                        Err(anyhow!("Access token was empty"))
                    }
                }
            }
            Err(e) => {
                println!("❌ [FCM] Google OAuth error after {:?}: {}", start.elapsed(), e);
                Err(anyhow!("Failed to get token: {}", e))
            }
        }
    }

    pub async fn send_to_user(
        &self,
        state: &AppState,
        user_id: &str,
        title: &str,
        body: &str,
        data: Value,
        notification_type: &str,
    ) -> Result<bool, AppError> {
        println!("\n📱 [FCM] ===== SENDING TO USER: {} =====", user_id);
        println!("📱 [FCM] Title: {}", title);
        println!("📱 [FCM] Type: {}", notification_type);

        let start = Instant::now();

        let tokens_collection: Collection<FCMToken> = state.db.collection("fcm_tokens");
        let filter = doc! { "user_id": user_id };

        println!("📱 [FCM] Querying database for user tokens...");
        let mut cursor = match tokens_collection.find(filter).await {
            Ok(c) => {
                println!("✅ [FCM] Database query successful");
                c
            }
            Err(e) => {
                println!("❌ [FCM] Database error: {}", e);
                return Err(AppError::InternalServerError(format!("Database error: {}", e)));
            }
        };

        let mut tokens_found = 0;
        let mut success = false;

        while let Some(token_doc) = cursor.try_next().await
            .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))? {

            tokens_found += 1;
            let token_preview = &token_doc.fcm_token[0..20.min(token_doc.fcm_token.len())];
            println!("📱 [FCM] Found token #{}: {}...", tokens_found, token_preview);
            println!("📱 [FCM] Platform: {}", token_doc.platform);

            if self.send_to_device(&token_doc.fcm_token, title, body, data.clone(), notification_type).await {
                success = true;
                println!("✅ [FCM] Successfully sent to token #{}", tokens_found);
            } else {
                println!("❌ [FCM] Failed to send to token #{}", tokens_found);
            }
        }

        if tokens_found == 0 {
            println!("⚠️ [FCM] No FCM tokens found for user: {}", user_id);
        } else {
            println!("📱 [FCM] Found {} total tokens for user", tokens_found);
        }

        println!("📱 [FCM] Saving notification to database...");
        if let Err(e) = self.save_notification(state, user_id, notification_type, title, body, data).await {
            println!("⚠️ [FCM] Failed to save notification: {}", e);
        } else {
            println!("✅ [FCM] Notification saved to database");
        }

        println!("📱 [FCM] Total time for user {}: {:?}", user_id, start.elapsed());
        println!("📱 [FCM] ===== END USER {} =====\n", user_id);

        Ok(success)
    }

    async fn send_to_device(
        &self,
        token: &str,
        title: &str,
        body: &str,
        data: Value,
        notification_type: &str,
    ) -> bool {
        println!("\n   🔧 [FCM-DEVICE] ===== SENDING TO DEVICE =====");
        let device_start = Instant::now();

        let token_preview = &token[0..20.min(token.len())];
        println!("   🔧 [FCM-DEVICE] Token: {}...", token_preview);
        println!("   🔧 [FCM-DEVICE] Title: {}", title);
        println!("   🔧 [FCM-DEVICE] Body: {}", body);
        println!("   🔧 [FCM-DEVICE] Type: {}", notification_type);

        // Step 1: Get access token
        println!("   🔧 [FCM-DEVICE] Step 1/4: Getting access token...");
        let token_start = Instant::now();
        let access_token = match self.get_access_token().await {
            Ok(t) => {
                println!("   ✅ [FCM-DEVICE] Access token obtained in {:?}", token_start.elapsed());
                t
            }
            Err(e) => {
                println!("   ❌ [FCM-DEVICE] Failed to get access token: {}", e);
                return false;
            }
        };

        // Step 2: Build FCM message
        println!("   🔧 [FCM-DEVICE] Step 2/4: Building FCM message...");
        let message_start = Instant::now();
        let message = json!({
            "message": {
                "token": token,
                "notification": {
                    "title": title,
                    "body": body,
                },
                "data": {
                    "type": notification_type,
                    "click_action": "FLUTTER_NOTIFICATION_CLICK",
                    "data": serde_json::to_string(&data).unwrap_or_default(),
                },
                "android": {
                    "priority": "high",
                    "notification": {
                        "click_action": "FLUTTER_NOTIFICATION_CLICK",
                        "channel_id": "vote_notifications",
                        "sound": "default",
                    }
                },
                "apns": {
                    "headers": {
                        "apns-priority": "10"
                    },
                    "payload": {
                        "aps": {
                            "sound": "default",
                            "badge": 1,
                            "category": "VOTE_CATEGORY"
                        }
                    }
                }
            }
        });
        println!("   ✅ [FCM-DEVICE] Message built in {:?}", message_start.elapsed());
        println!("   📤 [FCM-DEVICE] Message size: {} bytes", serde_json::to_string(&message).unwrap().len());

        // Step 3: Send to FCM API
        println!("   🔧 [FCM-DEVICE] Step 3/4: Sending to FCM API...");

        let _api_start = Instant::now();  // Add underscore
        let url = format!("https://fcm.googleapis.com/v1/projects/{}/messages:send", FIREBASE_PROJECT_ID);
        println!("   📤 [FCM-DEVICE] URL: {}", url);

        let request_start = Instant::now();
        let response_result = self.client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&message)
            .send()
            .await;

        match response_result {
            Ok(response) => {
                println!("   ✅ [FCM-DEVICE] HTTP request completed in {:?}", request_start.elapsed());
                let status = response.status();
                println!("   📥 [FCM-DEVICE] Response status: {} ({})", status, status.as_u16());

                // Step 4: Process response
                println!("   🔧 [FCM-DEVICE] Step 4/4: Processing response...");

                if status.is_success() {
                    println!("   ✅ [FCM-DEVICE] ✅ SUCCESS! Notification sent to FCM");
                    println!("   ✅ [FCM-DEVICE] Total device time: {:?}", device_start.elapsed());
                    println!("   🔧 [FCM-DEVICE] ===== END DEVICE =====\n");
                    true
                } else {
                    println!("   ❌ [FCM-DEVICE] FCM API error: {}", status);

                    // Try to read error body
                    match response.text().await {
                        Ok(error_text) => {
                            println!("   ❌ [FCM-DEVICE] Error details: {}", error_text);

                            // Check for common FCM errors
                            if error_text.contains("INVALID_ARGUMENT") {
                                println!("   ❌ [FCM-DEVICE] Invalid argument - check token format");
                            } else if error_text.contains("UNREGISTERED") {
                                println!("   ❌ [FCM-DEVICE] Token is unregistered - device uninstalled app");
                            } else if error_text.contains("SENDER_ID_MISMATCH") {
                                println!("   ❌ [FCM-DEVICE] Sender ID mismatch - wrong Firebase project");
                            } else if error_text.contains("QUOTA_EXCEEDED") {
                                println!("   ❌ [FCM-DEVICE] Quota exceeded - too many messages");
                            } else if error_text.contains("UNAVAILABLE") {
                                println!("   ❌ [FCM-DEVICE] Service temporarily unavailable");
                            } else if error_text.contains("INTERNAL") {
                                println!("   ❌ [FCM-DEVICE] Internal FCM server error");
                            }
                        }
                        Err(e) => {
                            println!("   ❌ [FCM-DEVICE] Could not read error body: {}", e);
                        }
                    }

                    println!("   ❌ [FCM-DEVICE] Total device time: {:?}", device_start.elapsed());
                    println!("   🔧 [FCM-DEVICE] ===== END DEVICE (FAILED) =====\n");
                    false
                }
            }
            Err(e) => {
                println!("   ❌ [FCM-DEVICE] HTTP request failed after {:?}: {}", request_start.elapsed(), e);

                // Check for specific network errors
                if e.is_timeout() {
                    println!("   ❌ [FCM-DEVICE] Request timed out - network issue");
                } else if e.is_connect() {
                    println!("   ❌ [FCM-DEVICE] Connection failed - cannot reach FCM servers");
                } else if e.is_request() {
                    println!("   ❌ [FCM-DEVICE] Invalid request - malformed");
                }

                println!("   ❌ [FCM-DEVICE] Total device time: {:?}", device_start.elapsed());
                println!("   🔧 [FCM-DEVICE] ===== END DEVICE (FAILED) =====\n");
                false
            }
        }
    }

    pub async fn send_to_multiple_users(
        &self,
        state: &AppState,
        user_ids: Vec<String>,
        title: &str,
        body: &str,
        data: Value,
        notification_type: &str,
    ) -> Result<usize, AppError> {
        println!("\n📱📱📱 [FCM-BULK] Sending to {} users", user_ids.len());
        let bulk_start = Instant::now();

        let mut success_count = 0;
        for (index, user_id) in user_ids.iter().enumerate() {
            println!("📱📱📱 [FCM-BULK] Processing user {}/{}", index + 1, user_ids.len());

            match self.send_to_user(state, user_id, title, body, data.clone(), notification_type).await {
                Ok(success) => {
                    if success {
                        success_count += 1;
                    }
                }
                Err(e) => {
                    println!("⚠️ [FCM-BULK] Error for user {}: {}", user_id, e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        println!("📱📱📱 [FCM-BULK] Complete: {}/{} successful in {:?}",
            success_count, user_ids.len(), bulk_start.elapsed());

        Ok(success_count)
    }

    async fn save_notification(
        &self,
        state: &AppState,
        user_id: &str,
        notification_type: &str,
        title: &str,
        body: &str,
        data: Value,
    ) -> Result<(), AppError> {
        println!("   💾 [FCM] Saving notification to database...");
        let save_start = Instant::now();

        let collection: Collection<Notification> = state.db.collection("notifications");
        let notification = Notification {
            id: None,
            user_id: user_id.to_string(),
            notification_type: notification_type.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            data,
            is_read: false,
            created_at: BsonDateTime::now(),
        };

        match collection.insert_one(notification).await {
            Ok(_) => {
                println!("   ✅ [FCM] Notification saved in {:?}", save_start.elapsed());
                Ok(())
            }
            Err(e) => {
                println!("   ❌ [FCM] Failed to save notification: {}", e);
                Err(AppError::InternalServerError(format!("Database error: {}", e)))
            }
        }
    }
}

pub async fn init_fcm_service() -> anyhow::Result<Arc<FCMService>> {
    println!("\n🚀🚀🚀 INITIALIZING FCM SERVICE 🚀🚀🚀");
    let init_start = Instant::now();

    let service = match FCMService::new().await {
        Ok(s) => {
            println!("✅ FCMService::new() completed successfully");
            s
        }
        Err(e) => {
            println!("❌ FCMService::new() failed: {}", e);
            return Err(e);
        }
    };

    // Test getting a token to verify everything works
    println!("🔍 Testing FCM with a sample token request...");
    match service.get_access_token().await {
        Ok(token) => {
            println!("✅ Test token obtained successfully! (preview: {}...)", &token[0..20.min(token.len())]);
        }
        Err(e) => {
            println!("⚠️ Test token request failed, but service may still work: {}", e);
        }
    }

    println!("✅ FCM Service initialized in {:?}", init_start.elapsed());
    println!("🚀🚀🚀 FCM SERVICE READY 🚀🚀🚀\n");

    Ok(Arc::new(service))
}
