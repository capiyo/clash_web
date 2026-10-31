use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::json;
use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use yup_oauth2::parse_service_account_key;

/// Server-side Firebase Storage access for feed and profile media.
///
/// The service account is deliberately configured separately from FCM: the
/// Storage bucket belongs to the frontend Firebase project (funzy-d56d7),
/// which may differ from the legacy FCM project used by existing deployments.
#[derive(Clone)]
pub struct FirebaseStorageService {
    bucket: String,
    project_id: String,
    client_email: String,
    private_key: String,
    client: Client,
    cached_token: Arc<Mutex<Option<(String, Instant)>>>,
}

impl FirebaseStorageService {
    /// Returns None when Storage credentials are not configured. This keeps
    /// existing API instances bootable while deployment secrets are prepared.
    pub fn from_env() -> Option<Self> {
        // Reuse the Firebase service-account credentials already used by
        // the application (FCM). Do not introduce a second credential set for
        // profile uploads. This is the existing web/Flutter Firebase bucket.
        let bucket = "funzy-d56d7.firebasestorage.app".to_string();
        let client_email = env::var("FIREBASE_CLIENT_EMAIL").ok()?;
        let private_key = env::var("FIREBASE_PRIVATE_KEY").ok()?;
        let project_id = env::var("FIREBASE_PROJECT_ID")
            .unwrap_or_else(|_| "clash-66865".to_string());

        if bucket.trim().is_empty()
            || client_email.trim().is_empty()
            || private_key.trim().is_empty()
            || project_id.trim().is_empty()
        {
            return None;
        }

        Some(Self {
            bucket,
            project_id,
            client_email,
            private_key,
            client: Client::new(),
            cached_token: Arc::new(Mutex::new(None)),
        })
    }

    async fn access_token(&self) -> Result<String> {
        let mut cached = self.cached_token.lock().await;
        if let Some((token, expires_at)) = cached.as_ref() {
            if Instant::now() < *expires_at {
                return Ok(token.clone());
            }
        }

        let private_key = self.private_key.replace("\\n", "\n");
        let service_account = json!({
            "type": "service_account",
            "project_id": self.project_id,
            "private_key": private_key,
            "client_email": self.client_email,
            "token_uri": "https://oauth2.googleapis.com/token"
        });
        let key = parse_service_account_key(service_account.to_string().as_bytes())
            .context("Could not parse Firebase Storage service-account credentials")?;
        let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(key)
            .build()
            .await
            .context("Could not initialize Firebase Storage authentication")?;
        let token = authenticator
            .token(&["https://www.googleapis.com/auth/devstorage.read_write"])
            .await
            .context("Could not obtain Firebase Storage access token")?
            .token()
            .ok_or_else(|| anyhow!("Firebase Storage returned an empty access token"))?
            .to_string();

        // Google OAuth access tokens normally live for one hour. Refresh early
        // so uploads do not fail at the expiry boundary.
        *cached = Some((token.clone(), Instant::now() + Duration::from_secs(3000)));
        Ok(token)
    }

    /// Uploads bytes to the configured Firebase bucket and returns the
    /// Firebase download URL together with the stable object path.
    pub async fn upload(
        &self,
        object_path: &str,
        bytes: &[u8],
        content_type: &str,
    ) -> Result<(String, String)> {
        if object_path.is_empty() || object_path.starts_with('/') || object_path.contains("..") {
            return Err(anyhow!("Invalid Firebase Storage object path"));
        }

        let access_token = self.access_token().await?;
        let upload_response = self
            .client
            .post(format!(
                "https://storage.googleapis.com/upload/storage/v1/b/{}/o",
                self.bucket
            ))
            .query(&[("uploadType", "media"), ("name", object_path)])
            .bearer_auth(&access_token)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes.to_vec())
            .send()
            .await
            .context("Firebase Storage upload request failed")?;

        if !upload_response.status().is_success() {
            let status = upload_response.status();
            let body = upload_response.text().await.unwrap_or_default();
            return Err(anyhow!("Firebase Storage upload failed ({status}): {body}"));
        }

        let download_token = uuid::Uuid::new_v4().to_string();
        let encoded_path = object_path.replace('/', "%2F");
        let metadata_response = self
            .client
            .patch(format!(
                "https://storage.googleapis.com/storage/v1/b/{}/o/{}",
                self.bucket, encoded_path
            ))
            .bearer_auth(&access_token)
            .json(&json!({
                "metadata": {
                    "firebaseStorageDownloadTokens": download_token
                }
            }))
            .send()
            .await
            .context("Firebase Storage metadata update failed")?;

        if !metadata_response.status().is_success() {
            let status = metadata_response.status();
            let body = metadata_response.text().await.unwrap_or_default();
            let _ = self.delete(object_path).await;
            return Err(anyhow!("Could not configure Firebase download URL ({status}): {body}"));
        }

        let url = format!(
            "https://firebasestorage.googleapis.com/v0/b/{}/o/{}?alt=media&token={}",
            self.bucket, encoded_path, download_token
        );
        Ok((url, object_path.to_string()))
    }

    pub async fn delete(&self, object_path: &str) -> Result<()> {
        if object_path.is_empty() {
            return Ok(());
        }
        let access_token = self.access_token().await?;
        let encoded_path = object_path.replace('/', "%2F");
        let response = self
            .client
            .delete(format!(
                "https://storage.googleapis.com/storage/v1/b/{}/o/{}",
                self.bucket, encoded_path
            ))
            .bearer_auth(access_token)
            .send()
            .await
            .context("Firebase Storage delete request failed")?;

        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(anyhow!("Firebase Storage delete failed ({status}): {body}"))
    }
}
