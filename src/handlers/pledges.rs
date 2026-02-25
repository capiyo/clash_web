use axum::{
    extract::{State, Query},
    response::Json,
};
use serde::Deserialize;
use chrono::Utc;
use mongodb::{Collection, bson::{doc, oid::ObjectId}};
use futures_util::TryStreamExt;

use crate::{
    state::AppState,
    errors::{AppError, Result},
    models::pledges::{Pledge, CreatePledge, PledgeQuery},
};

#[derive(Debug, Deserialize)]
pub struct PledgeStatsQuery {
    pub home_team: Option<String>,
    pub away_team: Option<String>,
}

// Get all pledges with optional filtering
pub async fn get_pledges(
    State(state): State<AppState>,
    Query(query): Query<PledgeQuery>,
) -> Result<Json<Vec<Pledge>>> {
    println!("🔍 GET /api/pledges called - Starting MongoDB query...");

    let collection: Collection<Pledge> = state.db.collection("pledges");

    // Build MongoDB filter
    let mut filter = doc! {};

    if let Some(username) = &query.username {
        filter.insert("username", username);
    }

    if let Some(phone) = &query.phone {
        filter.insert("phone", phone);
    }

    if let Some(home_team) = &query.home_team {
        filter.insert("home_team", home_team);
    }

    if let Some(away_team) = &query.away_team {
        filter.insert("away_team", away_team);
    }

    let cursor = collection.find(filter).await?;
    let mut pledges: Vec<Pledge> = cursor.try_collect().await?;

    // Sort by created_at descending
    pledges.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    println!("✅ Successfully fetched {} pledges", pledges.len());
    Ok(Json(pledges))
}

// Create a new pledge
pub async fn create_pledge(
    State(state): State<AppState>,
    Json(payload): Json<CreatePledge>,
) -> Result<Json<Pledge>> {
    println!("🎯 Creating new pledge for user: {}", payload.username);

    // Validate required fields
    if payload.username.is_empty() || payload.phone.is_empty() || payload.selection.is_empty() {
        return Err(AppError::InvalidUserData);
    }

    if payload.amount <= 0.0 {
        return Err(AppError::InvalidUserData);
    }

    let collection: Collection<Pledge> = state.db.collection("pledges");

    let pledge = Pledge {
        _id: Some(ObjectId::new()),
        username: payload.username.clone(),
        phone: payload.phone.clone(),
        selection: payload.selection.clone(),
        amount: payload.amount,
        time: Utc::now(),
        fan: payload.fan.clone(),
        home_team: payload.home_team.clone(),
        away_team: payload.away_team.clone(),
        starter_id: payload.starter_id.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Insert the pledge
    collection.insert_one(&pledge).await?;

    println!("✅ Successfully created pledge for user: {} - Amount: ₿{}", payload.username, payload.amount);
    Ok(Json(pledge))
}

// Get pledge statistics for a specific match
pub async fn get_pledge_stats(
    State(state): State<AppState>,
    Query(query): Query<PledgeStatsQuery>,
) -> Result<Json<serde_json::Value>> {
    println!("📊 Getting pledge statistics...");

    let (home_team, away_team) = match (&query.home_team, &query.away_team) {
        (Some(home), Some(away)) => (home, away),
        _ => return Err(AppError::InvalidUserData),
    };

    let collection: Collection<Pledge> = state.db.collection("pledges");

    // Build filter for the specific match
    let filter = doc! {
        "home_team": home_team,
        "away_team": away_team
    };

    // Get all pledges for this match
    let cursor = collection.find(filter.clone()).await?;
    let pledges: Vec<Pledge> = cursor.try_collect().await?;

    // Calculate statistics
    let total_pledges = pledges.len() as i64;
    let total_amount: f64 = pledges.iter().map(|p| p.amount).sum();

    // Count selections
    let home_pledges = pledges.iter().filter(|p| p.selection == "home_team").count() as i64;
    let away_pledges = pledges.iter().filter(|p| p.selection == "away_team").count() as i64;
    let draw_pledges = pledges.iter().filter(|p| p.selection == "draw").count() as i64;

    let stats = serde_json::json!({
        "total_pledges": total_pledges,
        "total_amount": total_amount,
        "selection_breakdown": {
            "home_team": home_pledges,
            "away_team": away_pledges,
            "draw": draw_pledges
        },
        "match": {
            "home_team": home_team,
            "away_team": away_team
        }
    });

    println!("✅ Successfully fetched pledge statistics");
    Ok(Json(stats))
}

// Get user's pledging history
pub async fn get_user_pledges(
    State(state): State<AppState>,
    Query(query): Query<PledgeQuery>,
) -> Result<Json<Vec<Pledge>>> {
    println!("👤 Getting user pledges...");

    let username = query.username.ok_or(AppError::InvalidUserData)?;

    let collection: Collection<Pledge> = state.db.collection("pledges");

    let filter = doc! { "username": &username };
    let cursor = collection.find(filter).await?;
    let mut pledges: Vec<Pledge> = cursor.try_collect().await?;

    pledges.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    println!("✅ Successfully fetched {} pledges for user", pledges.len());
    Ok(Json(pledges))
}

pub async fn get_recent_pledges(
    State(state): State<AppState>,
) -> Result<Json<Vec<Pledge>>> {
    println!("🕒 Getting recent pledges...");

    let collection: Collection<Pledge> = state.db.collection("pledges");

    let cursor = collection.find(doc! {}).await?;
    let mut pledges: Vec<Pledge> = cursor.try_collect().await?;

    pledges.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let recent_pledges: Vec<Pledge> = pledges.into_iter().take(10).collect();

    println!("✅ Successfully fetched {} recent pledges", recent_pledges.len());
    Ok(Json(recent_pledges))
}