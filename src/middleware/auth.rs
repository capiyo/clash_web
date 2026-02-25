use axum::{
    extract::Request,
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use crate::models::user::Claims;

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

    let token_data = decode::<Claims>(
        token,
        &decoding_key,
        &Validation::new(Algorithm::HS256),
    )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Insert claims into request extensions
    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}