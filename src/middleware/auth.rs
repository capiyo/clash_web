use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use crate::models::user::Claims;

#[derive(Debug, Deserialize)]
struct FirebaseTokenClaims {
    sub: String,
    exp: usize,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    phone_number: Option<String>,
}

async fn decode_firebase_token(token: &str) -> Result<Claims, StatusCode> {
    let project_id = std::env::var("FIREBASE_PROJECT_ID")
        .unwrap_or_else(|_| "clash-66865".to_string());
    let header = decode_header(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let kid = header.kid.ok_or(StatusCode::UNAUTHORIZED)?;

    let certs: std::collections::HashMap<String, String> = reqwest::Client::new()
        .get("https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com")
        .send()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .error_for_status()
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .json()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let certificate = certs.get(&kid).ok_or(StatusCode::UNAUTHORIZED)?;
    let key = DecodingKey::from_rsa_pem(certificate.as_bytes())
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let issuer = format!("https://securetoken.google.com/{}", project_id);
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[project_id.as_str()]);
    validation.set_issuer(&[issuer.as_str()]);

    let decoded = decode::<FirebaseTokenClaims>(token, &key, &validation)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if decoded.claims.sub.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(Claims {
        sub: decoded.claims.sub,
        username: decoded.claims.name.unwrap_or_default(),
        phone: decoded.claims.phone_number.unwrap_or_default(),
        exp: decoded.claims.exp,
    })
}

/// Accept the backend's existing HS256 token and Firebase ID tokens used by
/// the web/mobile clients. The avatar handler still checks ownership against
/// the requested account before writing anything.
pub async fn auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = headers
        .get("authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let decoding_key = DecodingKey::from_secret(secret.as_ref());

    let claims = match decode::<Claims>(
        token,
        &decoding_key,
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(token_data) => token_data.claims,
        Err(_) => decode_firebase_token(token).await?,
    };

    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}
