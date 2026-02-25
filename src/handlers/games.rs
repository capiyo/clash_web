use axum::{
    extract::{State, Query, Path},
    response::Json,
};
use serde::Deserialize;
use mongodb::bson::{doc, oid::ObjectId, DateTime as BsonDateTime};
use chrono::Utc;
use mongodb::Collection;
use futures_util::TryStreamExt;

use crate::state::AppState;
use crate::models::game::{Game, LiveUpdate, LiveGamesResponse, GameQuery as ModelGameQuery};
use crate::errors::{AppError, Result};

#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub status: Option<String>,
    pub league: Option<String>,
    pub home_team: Option<String>,
    pub away_team: Option<String>,
    pub is_live: Option<bool>,
}

pub async fn get_games(
    State(state): State<AppState>,
    Query(query): Query<GameQuery>,
) -> Result<Json<Vec<Game>>> {
    println!("🔍 GET /api/games called with query: {:?}", query);
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");
    let mut filter = doc! {};

    if let Some(status) = &query.status {
        filter.insert("status", status);
        println!("   → Filtering by status: {}", status);
    }
    if let Some(league) = &query.league {
        filter.insert("league", league);
        println!("   → Filtering by league: {}", league);
    }
    if let Some(home_team) = &query.home_team {
        filter.insert("home_team", home_team);
        println!("   → Filtering by home team: {}", home_team);
    }
    if let Some(away_team) = &query.away_team {
        filter.insert("away_team", away_team);
        println!("   → Filtering by away team: {}", away_team);
    }
    if let Some(is_live) = query.is_live {
        filter.insert("is_live", is_live);
        println!("   → Filtering by is_live: {}", is_live);
    }

    println!("   → Database filter: {:?}", filter);

    let cursor = collection.find(filter).await?;
    let mut games: Vec<Game> = cursor.try_collect().await?;

    println!("   → Fetched {} games from database", games.len());

    // Sort by scraped_at descending (most recent first) - CHANGED HERE
    games.sort_by(|a, b| b.scraped_at.cmp(&a.scraped_at));

    let elapsed = start_time.elapsed();
    println!("✅ Successfully fetched {} games in {:?}", games.len(), elapsed);
    Ok(Json(games))
}

pub async fn get_game_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Game>> {
    println!("🔍 GET /api/games/{} called", id);
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    println!("   → Using ID as string: {}", id);
    // Don't parse as ObjectId - use string ID directly
    let filter = doc! { "_id": &id };
    println!("   → Database filter: {:?}", filter);

    match collection.find_one(filter).await? {
        Some(game) => {
            let elapsed = start_time.elapsed();
            println!("✅ Found game: {} vs {} in {:?}", game.home_team, game.away_team, elapsed);
            Ok(Json(game))
        }
        None => {
            let elapsed = start_time.elapsed();
            println!("❌ Game not found: {} (searched in {:?})", id, elapsed);
            Err(AppError::DocumentNotFound)
        }
    }
}

pub async fn get_game_by_match_id(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<Game>> {
    println!("🔍 GET /api/games/match/{} called", match_id);
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    let filter = doc! { "match_id": &match_id };
    println!("   → Database filter: {:?}", filter);

    match collection.find_one(filter).await? {
        Some(game) => {
            let elapsed = start_time.elapsed();
            println!("✅ Found game by match_id: {} vs {} in {:?}",
                     game.home_team, game.away_team, elapsed);
            Ok(Json(game))
        }
        None => {
            let elapsed = start_time.elapsed();
            println!("❌ Game not found with match_id: {} (searched in {:?})", match_id, elapsed);
            Err(AppError::DocumentNotFound)
        }
    }
}

pub async fn get_live_games(
    State(state): State<AppState>,
) -> Result<Json<LiveGamesResponse>> {
    println!("🔥 GET /api/games/live called");
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    let filter = doc! {
        "status": "live",
        "is_live": true
    };
    println!("   → Database filter: {:?}", filter);

    let cursor = collection.find(filter).await?;
    let live_games: Vec<Game> = cursor.try_collect().await?;

    println!("   → Fetched {} live games from database", live_games.len());

    // Get count BEFORE moving live_games
    let count = live_games.len();

    // Get current time as fallback
    let current_time = BsonDateTime::from_chrono(Utc::now());

    // Find max timestamp using scraped_at instead of last_updated - CHANGED HERE
    let max_timestamp = live_games.iter()
        .map(|g| g.scraped_at.timestamp_millis())
        .max()
        .unwrap_or_else(|| current_time.timestamp_millis());

    // Create new BsonDateTime from milliseconds
    let last_updated = BsonDateTime::from_millis(max_timestamp);

    println!("   → Most recent game timestamp: {} ms", max_timestamp);

    // Create response - now live_games is only used once
    let response = LiveGamesResponse {
        live_games,  // This moves live_games into the response
        count,
        last_updated,
    };

    let elapsed = start_time.elapsed();
    println!("✅ Successfully fetched {} live games in {:?}", count, elapsed);
    Ok(Json(response))
}

pub async fn get_upcoming_games(
    State(state): State<AppState>,
) -> Result<Json<Vec<Game>>> {
    println!("⏳ GET /api/games/upcoming called");
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    let filter = doc! {
        "status": "upcoming"
    };
    println!("   → Database filter: {:?}", filter);

    let cursor = collection.find(filter).await?;
    let mut games: Vec<Game> = cursor.try_collect().await?;

    println!("   → Fetched {} upcoming games from database", games.len());

    // Sort by date and time (earliest first)
    games.sort_by(|a, b| {
        let date_time_a = format!("{} {}", a.date, a.time);
        let date_time_b = format!("{} {}", b.date, b.time);
        date_time_a.cmp(&date_time_b)
    });

    println!("   → Sorted games by date/time");

    let elapsed = start_time.elapsed();
    println!("✅ Successfully fetched {} upcoming games in {:?}", games.len(), elapsed);
    Ok(Json(games))
}

pub async fn update_game_score(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
    Json(payload): Json<LiveUpdate>,
) -> Result<Json<Game>> {
    println!("📝 PATCH /api/games/{}/score called", match_id);
    println!("   → Payload: {:?}", payload);
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    let filter = doc! { "match_id": &match_id };
    println!("   → Database filter: {:?}", filter);

    let mut update_doc = doc! {};

    if let Some(home_score) = payload.home_score {
        update_doc.insert("home_score", home_score);
        println!("   → Setting home_score: {}", home_score);
    }

    if let Some(away_score) = payload.away_score {
        update_doc.insert("away_score", away_score);
        println!("   → Setting away_score: {}", away_score);
    }

    // Always update scraped_at timestamp - CHANGED HERE
    let new_timestamp = BsonDateTime::from_chrono(Utc::now());
    update_doc.insert("scraped_at", new_timestamp);
    println!("   → Updating scraped_at timestamp");

    let update = doc! {
        "$set": update_doc
    };
    println!("   → Update operation: {:?}", update);

    // First, update the document
    println!("   → Executing database update...");
    let update_result = collection.update_one(filter.clone(), update).await?;
    println!("   → Update result: {:?}", update_result);

    if update_result.matched_count == 0 {
        let elapsed = start_time.elapsed();
        println!("❌ Game not found with match_id: {} (searched in {:?})", match_id, elapsed);
        return Err(AppError::DocumentNotFound);
    }

    println!("   → Matched {} document(s), modified {} document(s)",
             update_result.matched_count, update_result.modified_count);

    // Then, fetch the updated document
    println!("   → Fetching updated document...");
    match collection.find_one(filter).await? {
        Some(game) => {
            let elapsed = start_time.elapsed();
            println!("✅ Updated game {} score to {}-{} in {:?}",
                     match_id,
                     game.home_score.unwrap_or(0),
                     game.away_score.unwrap_or(0),
                     elapsed);
            Ok(Json(game))
        }
        None => {
            let elapsed = start_time.elapsed();
            println!("❌ Game not found after update: {} (operation took {:?})", match_id, elapsed);
            Err(AppError::DocumentNotFound)
        }
    }
}

pub async fn get_game_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    println!("📊 GET /api/games/stats called");
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    // Get all games
    println!("   → Fetching all games from database...");
    let cursor = collection.find(doc! {}).await?;
    let games: Vec<Game> = cursor.try_collect().await?;
    println!("   → Retrieved {} total games", games.len());

    // Calculate statistics
    let total_games = games.len() as i64;
    println!("   → Total games: {}", total_games);

    // Count by status (using your database status values)
    let upcoming_games = games.iter()
        .filter(|g| g.status == "upcoming")
        .count() as i64;
    println!("   → Upcoming games: {}", upcoming_games);

    let live_games = games.iter()
        .filter(|g| g.status == "live")
        .count() as i64;
    println!("   → Live games: {}", live_games);

    let completed_games = games.iter()
        .filter(|g| g.status == "completed")
        .count() as i64;
    println!("   → Completed games: {}", completed_games);

    // Count by league
    println!("   → Calculating league distribution...");
    use std::collections::HashMap;
    let mut league_counts: HashMap<String, i64> = HashMap::new();

    for game in &games {
        *league_counts.entry(game.league.clone()).or_insert(0) += 1;
    }

    println!("   → Found {} unique leagues", league_counts.len());

    let league_stats: Vec<_> = league_counts.into_iter()
        .map(|(league, count)| serde_json::json!({
            "league": league,
            "count": count
        }))
        .collect();

    // Calculate average odds
    println!("   → Calculating average odds...");
    let total_games_f64 = total_games as f64;
    let avg_home_win = if total_games > 0 {
        games.iter().map(|g| g.home_win).sum::<f64>() / total_games_f64
    } else { 0.0 };

    let avg_away_win = if total_games > 0 {
        games.iter().map(|g| g.away_win).sum::<f64>() / total_games_f64
    } else { 0.0 };

    let avg_draw = if total_games > 0 {
        games.iter().map(|g| g.draw).sum::<f64>() / total_games_f64
    } else { 0.0 };

    println!("   → Average odds - Home: {:.2}, Away: {:.2}, Draw: {:.2}",
             avg_home_win, avg_away_win, avg_draw);

    // Prepare recent games
    println!("   → Preparing recent games list...");
    let recent_games_json: Vec<_> = games.iter()
        .take(5)
        .map(|g| serde_json::json!({
            "match_id": g.match_id.clone(),
            "match": format!("{} vs {}", g.home_team, g.away_team),
            "league": g.league.clone(),
            "status": g.status.clone(),
            "is_live": g.is_live,
            "date": g.date.clone(),
            "time": g.time.clone(),
            "score": if let (Some(h), Some(a)) = (g.home_score, g.away_score) {
                format!("{}-{}", h, a)
            } else {
                "TBD".to_string()
            },
            "odds": {
                "home_win": g.home_win,
                "away_win": g.away_win,
                "draw": g.draw
            }
        }))
        .collect();

    println!("   → Included {} recent games in response", recent_games_json.len());

    let stats = serde_json::json!({
        "total_games": total_games,
        "by_status": {
            "upcoming": upcoming_games,
            "live": live_games,
            "completed": completed_games
        },
        "by_league": league_stats,
        "average_odds": {
            "home_win": avg_home_win,
            "away_win": avg_away_win,
            "draw": avg_draw
        },
        "recent_games": recent_games_json
    });

    let elapsed = start_time.elapsed();
    println!("✅ Successfully fetched game statistics in {:?}", elapsed);
    Ok(Json(stats))
}

pub async fn get_recent_games(
    State(state): State<AppState>,
) -> Result<Json<Vec<Game>>> {
    println!("🕒 GET /api/games/recent called");
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    println!("   → Fetching all games...");
    let cursor = collection.find(doc! {}).await?;
    let mut games: Vec<Game> = cursor.try_collect().await?;
    println!("   → Retrieved {} games from database", games.len());

    // Sort by scraped_at descending (most recent first) - CHANGED HERE
    games.sort_by(|a, b| b.scraped_at.cmp(&a.scraped_at));
    println!("   → Sorted games by scraped_at timestamp");

    // Take only last 10
    let recent_games: Vec<Game> = games.into_iter().take(10).collect();
    println!("   → Limited to {} most recent games", recent_games.len());

    let elapsed = start_time.elapsed();
    println!("✅ Successfully fetched {} recent games in {:?}", recent_games.len(), elapsed);
    Ok(Json(recent_games))
}

pub async fn update_game_status(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Game>> {
    println!("📝 PATCH /api/games/{}/status called", match_id);
    println!("   → Payload: {:?}", payload);
    let start_time = std::time::Instant::now();

    let collection: Collection<Game> = state.db.collection("games");

    let status = payload.get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            println!("❌ Status field missing in payload");
            AppError::invalid_data("Status is required")
        })?;

    println!("   → Requested status update to: {}", status);

    // Validate status value (using your database status values)
    let valid_statuses = vec!["upcoming", "live", "completed"];
    if !valid_statuses.contains(&status) {
        let elapsed = start_time.elapsed();
        println!("❌ Invalid status '{}'. Must be one of: {:?} (checked in {:?})",
                 status, valid_statuses, elapsed);
        return Err(AppError::invalid_data(
            &format!("Invalid status. Must be one of: {:?}", valid_statuses)
        ));
    }

    let filter = doc! { "match_id": &match_id };
    println!("   → Database filter: {:?}", filter);

    // Update is_live based on status
    let is_live = status == "live";
    println!("   → Setting is_live to: {}", is_live);

    let update_timestamp = BsonDateTime::from_chrono(Utc::now());
    let update = doc! {
        "$set": {
            "status": status,
            "is_live": is_live,
            "scraped_at": update_timestamp  // CHANGED HERE from last_updated to scraped_at
        }
    };
    println!("   → Update operation: {:?}", update);

    // First, update the document
    println!("   → Executing database update...");
    let update_result = collection.update_one(filter.clone(), update).await?;
    println!("   → Update result: {:?}", update_result);

    if update_result.matched_count == 0 {
        let elapsed = start_time.elapsed();
        println!("❌ Game not found with match_id: {} (searched in {:?})", match_id, elapsed);
        return Err(AppError::DocumentNotFound);
    }

    println!("   → Matched {} document(s), modified {} document(s)",
             update_result.matched_count, update_result.modified_count);

    // Then, fetch the updated document
    println!("   → Fetching updated document...");
    match collection.find_one(filter).await? {
        Some(game) => {
            let elapsed = start_time.elapsed();
            println!("✅ Updated game {} to status: {} in {:?}", match_id, status, elapsed);
            Ok(Json(game))
        }
        None => {
            let elapsed = start_time.elapsed();
            println!("❌ Game not found after update: {} (operation took {:?})", match_id, elapsed);
            Err(AppError::DocumentNotFound)
        }
    }
}
