use axum::{
    extract::State,
    response::Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::Utc;
use mongodb::Collection;
use mongodb::bson::{doc, oid::ObjectId};
use futures_util::TryStreamExt;

use crate::state::AppState;
use crate::errors::{AppError, Result};
use crate::models::user::{
    User, CreateUser, LoginUser, LoginWithPhone, UserResponse, AuthResponse, Claims
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,  // This is 'payload', not 'req'
) -> Result<Json<AuthResponse>> {
    let collection: Collection<User> = state.db.collection("users");

    // Check if user exists by username or phone
    let filter = doc! {
        "$or": [
            { "username": &payload.username },
            { "phone": &payload.phone }
        ]
    };

    let existing_user = collection.find_one(filter).await?;

    if existing_user.is_some() {
        return Err(AppError::InvalidUserData);
    }

    // Hash password
    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_e| AppError::InvalidUserData)?;

    // Create user document - FIXED: Use 'payload' not 'req'
    let user = User {
        _id: None,
        username: payload.username.clone(),  // Fixed
        phone: payload.phone.clone(),        // Fixed
        password_hash: password_hash,        // Fixed
        balance: 0.0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        reset_otp: None,
    };

    // Insert user
    let _insert_result = collection.insert_one(&user).await?;

    // Get the inserted ID
    let inserted_id = user._id.unwrap();

    // Generate JWT token
    let user_response = UserResponse {
        id: inserted_id.to_hex(),
        username: payload.username.clone(),
        phone: payload.phone.clone(),
        balance: 0.0,
    };

    let claims = Claims {
        sub: inserted_id.to_hex(),
        username: payload.username.clone(),
        phone: payload.phone.clone(),
        exp: (Utc::now().timestamp() + 86400) as usize, // 24 hours
    };

    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "secret".to_string());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
        .map_err(|_| AppError::InvalidUserData)?;

    Ok(Json(AuthResponse {
        user: user_response,
        token,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginUser>,
) -> Result<Json<AuthResponse>> {
    let collection: Collection<User> = state.db.collection("users");

    // Find user by username
    let filter = doc! { "username": &payload.username };
    let user = collection.find_one(filter).await?
        .ok_or(AppError::InvalidUserData)?;

    // Verify password
    let valid = verify(&payload.password, &user.password_hash)
        .map_err(|_| AppError::InvalidUserData)?;

    if !valid {
        return Err(AppError::InvalidUserData);
    }

    // Generate JWT token
    let user_response = UserResponse {
        id: user._id.unwrap().to_hex(),
        username: user.username.clone(),
        phone: user.phone.clone(),
        balance: user.balance,
    };

    let claims = Claims {
        sub: user._id.unwrap().to_hex(),
        username: user.username.clone(),
        phone: user.phone.clone(),
        exp: (Utc::now().timestamp() + 86400) as usize,
    };

    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "secret".to_string());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
        .map_err(|_| AppError::InvalidUserData)?;

    Ok(Json(AuthResponse {
        user: user_response,
        token,
    }))
}

pub async fn login_with_phone(
    State(state): State<AppState>,
    Json(payload): Json<LoginWithPhone>,
) -> Result<Json<AuthResponse>> {
    let collection: Collection<User> = state.db.collection("users");

    // Find user by phone
    let filter = doc! { "phone": &payload.phone };
    let user = collection.find_one(filter).await?
        .ok_or(AppError::InvalidUserData)?;

    // Verify password
    let valid = verify(&payload.password, &user.password_hash)
        .map_err(|_| AppError::InvalidUserData)?;

    if !valid {
        return Err(AppError::InvalidUserData);
    }

    // Generate JWT token
    let user_response = UserResponse {
        id: user._id.unwrap().to_hex(),
        username: user.username.clone(),
        phone: user.phone.clone(),
        balance: user.balance,
    };

    let claims = Claims {
        sub: user._id.unwrap().to_hex(),
        username: user.username.clone(),
        phone: user.phone.clone(),
        exp: (Utc::now().timestamp() + 86400) as usize,
    };

    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "secret".to_string());

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
        .map_err(|_| AppError::InvalidUserData)?;

    Ok(Json(AuthResponse {
        user: user_response,
        token,
    }))
}

pub async fn get_user_profile(
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<UserResponse>> {
    let collection: Collection<User> = state.db.collection("users");

    let object_id = ObjectId::parse_str(&user_id)
        .map_err(|_| AppError::invalid_data("Invalid user ID"))?;

    let filter = doc! { "_id": object_id };
    let user = collection.find_one(filter).await?
        .ok_or(AppError::DocumentNotFound)?;

    let user_response = UserResponse {
        id: user._id.unwrap().to_hex(),
        username: user.username,
        phone: user.phone,
        balance: user.balance,
    };

    Ok(Json(user_response))
}

pub async fn get_all_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>> {
    let collection: Collection<User> = state.db.collection("users");

    let cursor = collection.find(doc! {}).await?;
    let users: Vec<User> = cursor.try_collect().await?;

    let user_responses: Vec<UserResponse> = users.into_iter()
        .map(|user| UserResponse {
            id: user._id.unwrap().to_hex(),
            username: user.username,
            phone: user.phone,
            balance: user.balance,
        })
        .collect();

    Ok(Json(user_responses))
}
