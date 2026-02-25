use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use bson::{doc, oid::ObjectId, DateTime as BsonDateTime};
use chrono::{Duration, Utc};
use futures_util::TryStreamExt;
use mongodb::{options::FindOptions, Collection};
use serde_json::json;
use validator::Validate;
use crate::services::fcm_service;

use crate::{
    errors::{AppError, Result},
    models::game::Game,
    models::vote::{
        parse_iso_timestamp_or_now, validate_selection,
        BulkVoteRequest, BulkVoteResponse, Comment, CommentQuery,
        CommentResponse, CommentStats, CreateComment, CreateLike, CreateVote,
        FixtureCountsResponse, FixtureStats, Like, LikeResponse, LikeStats, TotalCountsResponse,
        UserVoteStatus, Vote, VoteQuery, VoteResponse, VoteStats,
    },
    state::AppState,
};

// ========== VOTE HANDLERS ==========

pub async fn create_vote(
    State(state): State<AppState>,
    Json(payload): Json<CreateVote>,
) -> Result<Json<VoteResponse>> {
    println!("🗳️ Creating vote for user: {} ({})", payload.username, payload.voter_id);

    // Validate payload
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Validate selection
    validate_selection(&payload.selection).map_err(|e| AppError::ValidationError(e))?;

    // Validate draw field is "draw"
    if payload.draw != "draw" {
        return Err(AppError::ValidationError(
            "draw field must be 'draw'".to_string(),
        ));
    }

    // Validate fixture_id is provided
    if payload.fixture_id.trim().is_empty() {
        return Err(AppError::ValidationError(
            "fixtureId is required".to_string(),
        ));
    }

    // Check if user already voted for this fixture
    let vote_collection: Collection<Vote> = state.db.collection("votes");

    let existing_vote_filter = doc! {
        "voterId": &payload.voter_id,
        "fixture_id": &payload.fixture_id,
    };

    let existing_vote = vote_collection.find_one(existing_vote_filter).await?;

    if existing_vote.is_some() {
        return Ok(Json(VoteResponse {
            success: false,
            message: "User already voted for this fixture".to_string(),
            vote_id: None,
            data: None,
        }));
    }

    // Create vote document
    let vote = Vote {
        id: None,
        voter_id: payload.voter_id.clone(),
        username: payload.username.clone(),
        fixture_id: payload.fixture_id.clone(),
        home_team: payload.home_team.clone(),
        away_team: payload.away_team.clone(),
        draw: payload.draw.clone(),
        selection: payload.selection.clone(),
        vote_timestamp: BsonDateTime::from_chrono(Utc::now()),
        created_at: Some(BsonDateTime::from_chrono(Utc::now())),
    };

    // Insert into database
    let insert_result = vote_collection.insert_one(vote).await?;
    let vote_id = insert_result.inserted_id.as_object_id().unwrap().to_hex();

    // Fetch the inserted vote
    let filter = doc! { "_id": insert_result.inserted_id };
    let inserted_vote = vote_collection
        .find_one(filter)
        .await?
        .ok_or_else(|| AppError::DocumentNotFound)?;

    println!("✅ Vote created successfully: {} by {}", vote_id, payload.username);

    // ===== SEND FCM NOTIFICATIONS TO SUPPORTERS AND RIVALS =====
    let state_clone = state.clone();
    let payload_clone = payload.clone();

    tokio::spawn(async move {
        // Initialize FCM service
        if let Ok(fcm_service) = fcm_service::init_fcm_service().await {

            // Get all other votes for this fixture
            let other_votes_filter = doc! {
                "fixture_id": &payload_clone.fixture_id,
                "voterId": { "$ne": &payload_clone.voter_id }
            };

            let vote_collection: Collection<Vote> = state_clone.db.collection("votes");

            match vote_collection.find(other_votes_filter).await {
                Ok(cursor) => {
                    let other_votes: Vec<Vote> = cursor.try_collect().await.unwrap_or_default();

                    let mut supporter_ids = Vec::new();
                    let mut rival_ids = Vec::new();

                    // Categorize other voters
                    for vote in other_votes {
                        if vote.selection == payload_clone.selection {
                            supporter_ids.push(vote.voter_id);
                        } else {
                            rival_ids.push(vote.voter_id);
                        }
                    }

                    let fixture_name = format!("{} vs {}", payload_clone.home_team, payload_clone.away_team);
                    let vote_text = payload_clone.selection.replace("_", " ");

                    // Notify SUPPORTERS (people who voted the same way)
                    if !supporter_ids.is_empty() {
                        println!("📱 Notifying {} supporters", supporter_ids.len());
                        let _ = fcm_service.send_to_multiple_users(
                            &state_clone,
                            supporter_ids,
                            "🎉 New supporter joined you!",
                            &format!("{} also voted {} in {}",
                                payload_clone.username,
                                vote_text,
                                fixture_name
                            ),
                            serde_json::json!({
                                "fixture_id": payload_clone.fixture_id,
                                "voter_id": payload_clone.voter_id,
                                "voter_username": payload_clone.username,
                                "voter_selection": payload_clone.selection,
                                "home_team": payload_clone.home_team,
                                "away_team": payload_clone.away_team,
                                "type": "vote_supporter",
                                "action": "new_supporter"
                            }),
                            "vote_supporter"
                        ).await;
                    }

                    // Notify RIVALS (people who voted differently)
                    if !rival_ids.is_empty() {
                        println!("📱 Notifying {} rivals", rival_ids.len());
                        let _ = fcm_service.send_to_multiple_users(
                            &state_clone,
                            rival_ids,
                            "⚔️ Someone voted against you!",
                            &format!("{} voted {} in {}",
                                payload_clone.username,
                                vote_text,
                                fixture_name
                            ),
                            serde_json::json!({
                                "fixture_id": payload_clone.fixture_id,
                                "voter_id": payload_clone.voter_id,
                                "voter_username": payload_clone.username,
                                "voter_selection": payload_clone.selection,
                                "home_team": payload_clone.home_team,
                                "away_team": payload_clone.away_team,
                                "type": "vote_rival",
                                "action": "new_rival"
                            }),
                            "vote_rival"
                        ).await;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error finding other votes: {}", e);
                }
            }
        }
    });

    Ok(Json(VoteResponse {
        success: true,
        message: "Vote submitted successfully".to_string(),
        vote_id: Some(vote_id),
        data: Some(inserted_vote),
    }))
}

pub async fn get_votes(
    State(state): State<AppState>,
    Query(query): Query<VoteQuery>,
) -> Result<Json<Vec<Vote>>> {
    println!("🔍 Getting votes...");

    let collection: Collection<Vote> = state.db.collection("votes");
    let mut filter = doc! {};

    if let Some(fixture_id) = &query.fixture_id {
        filter.insert("fixture_id", fixture_id);
    }

    if let Some(voter_id) = &query.voter_id {
        filter.insert("voterId", voter_id);
    }

    let _options = FindOptions::builder()
        .sort(doc! { "vote_timestamp": -1 })
        .build();

    let cursor = collection.find(filter).await?;
    let votes: Vec<Vote> = cursor.try_collect().await?;

    println!("✅ Found {} votes", votes.len());
    Ok(Json(votes))
}

pub async fn bulk_create_votes(
    State(state): State<AppState>,
    Json(payload): Json<BulkVoteRequest>,
) -> Result<Json<BulkVoteResponse>> {
    println!("📦 Creating bulk votes ({} votes)", payload.votes.len());

    let collection: Collection<Vote> = state.db.collection("votes");
    let mut failed_votes = Vec::new();
    let mut votes_to_insert = Vec::new();
    let now = BsonDateTime::from_chrono(Utc::now());

    // Validate and prepare votes
    for (index, vote_data) in payload.votes.into_iter().enumerate() {
        match vote_data.validate() {
            Ok(_) => {
                // Validate draw field is "draw"
                if vote_data.draw != "draw" {
                    failed_votes.push(crate::models::vote::FailedVote {
                        index,
                        error: "draw field must be 'draw'".to_string(),
                        vote_data,
                    });
                    continue;
                }

                // Validate fixture_id is provided
                if vote_data.fixture_id.trim().is_empty() {
                    failed_votes.push(crate::models::vote::FailedVote {
                        index,
                        error: "fixtureId is required".to_string(),
                        vote_data,
                    });
                    continue;
                }

                let vote = Vote {
                    id: None,
                    voter_id: vote_data.voter_id.clone(),
                    username: vote_data.username.clone(),
                    fixture_id: vote_data.fixture_id.clone(),
                    home_team: vote_data.home_team.clone(),
                    away_team: vote_data.away_team.clone(),
                    draw: vote_data.draw.clone(),
                    selection: vote_data.selection.clone(),
                    vote_timestamp: now,
                    created_at: Some(now),
                };

                votes_to_insert.push(vote);
            }
            Err(e) => {
                failed_votes.push(crate::models::vote::FailedVote {
                    index,
                    error: e.to_string(),
                    vote_data,
                });
            }
        }
    }

    // Insert valid votes
    let inserted_count = if !votes_to_insert.is_empty() {
        let result = collection.insert_many(votes_to_insert).await?;
        result.inserted_ids.len() as u64
    } else {
        0
    };

    let failed_count = failed_votes.len() as u64;

    println!(
        "✅ Bulk vote creation: {} inserted, {} failed",
        inserted_count, failed_count
    );

    Ok(Json(BulkVoteResponse {
        success: true,
        inserted_count,
        failed_count,
        failed_votes,
    }))
}

pub async fn get_user_votes(
    State(state): State<AppState>,
    Path(voter_id): Path<String>,
) -> Result<Json<Vec<Vote>>> {
    println!("🔍 Getting votes for user: {}", voter_id);

    let collection: Collection<Vote> = state.db.collection("votes");
    let filter = doc! { "voterId": voter_id };

    let _options = FindOptions::builder()
        .sort(doc! { "vote_timestamp": -1 })
        .build();

    let cursor = collection.find(filter).await?;
    let votes: Vec<Vote> = cursor.try_collect().await?;

    println!("✅ Found {} votes for user", votes.len());
    Ok(Json(votes))
}

pub async fn get_fixture_votes(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<VoteStats>> {
    println!("📊 Getting vote stats for fixture: {}", fixture_id);

    let collection: Collection<Vote> = state.db.collection("votes");
    let filter = doc! { "fixture_id": &fixture_id };

    // Get all votes for this fixture
    let cursor = collection.find(filter).await?;
    let votes: Vec<Vote> = cursor.try_collect().await?;

    // Calculate statistics
    let total_votes = votes.len() as i64;

    let home_votes = votes.iter().filter(|v| v.selection == "home_team").count() as i64;

    let draw_votes = votes.iter().filter(|v| v.selection == "draw").count() as i64;

    let away_votes = votes.iter().filter(|v| v.selection == "away_team").count() as i64;

    // Get home_team and away_team from first vote (if exists)
    let (home_team, away_team) = if let Some(first_vote) = votes.first() {
        (first_vote.home_team.clone(), first_vote.away_team.clone())
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
    };

    // Calculate percentages
    let home_percentage = if total_votes > 0 {
        (home_votes as f64 / total_votes as f64) * 100.0
    } else {
        0.0
    };

    let draw_percentage = if total_votes > 0 {
        (draw_votes as f64 / total_votes as f64) * 100.0
    } else {
        0.0
    };

    let away_percentage = if total_votes > 0 {
        (away_votes as f64 / total_votes as f64) * 100.0
    } else {
        0.0
    };

    let stats = VoteStats {
        fixture_id: fixture_id.clone(),
        home_team,
        away_team,
        total_votes,
        home_votes,
        draw_votes,
        away_votes,
        home_percentage,
        draw_percentage,
        away_percentage,
    };

    println!("✅ Vote stats: {} votes total", total_votes);
    Ok(Json(stats))
}

pub async fn get_total_votes_for_fixture(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    println!("📊 Getting total vote count for fixture: {}", fixture_id);

    let collection: Collection<Vote> = state.db.collection("votes");
    let filter = doc! { "fixture_id": &fixture_id };

    let total_votes = collection.count_documents(filter).await? as i64;

    let response = json!({
        "success": true,
        "fixture_id": fixture_id,
        "total_votes": total_votes,
        "timestamp": Utc::now().to_rfc3339(),
    });

    println!("✅ Total votes for fixture {}: {}", fixture_id, total_votes);
    Ok(Json(response))
}

pub async fn get_user_vote_for_fixture(
    State(state): State<AppState>,
    Path((fixture_id, voter_id)): Path<(String, String)>,
) -> Result<Json<Option<Vote>>> {
    println!(
        "🔍 Checking user vote: {} for fixture: {}",
        voter_id, fixture_id
    );

    let collection: Collection<Vote> = state.db.collection("votes");
    let filter = doc! {
        "voterId": voter_id,
        "fixture_id": fixture_id,
    };

    let vote = collection.find_one(filter).await?;

    if vote.is_some() {
        println!("✅ User has voted for this fixture");
    } else {
        println!("❌ User has not voted for this fixture");
    }

    Ok(Json(vote))
}

pub async fn delete_vote(
    State(state): State<AppState>,
    Path(vote_id): Path<String>,
) -> Result<Json<VoteResponse>> {
    println!("🗑️ Deleting vote: {}", vote_id);

    let collection: Collection<Vote> = state.db.collection("votes");

    let object_id = ObjectId::parse_str(&vote_id)
        .map_err(|_| AppError::invalid_data("Invalid vote ID format"))?;

    let filter = doc! { "_id": object_id };

    let delete_result = collection.delete_one(filter).await?;

    if delete_result.deleted_count == 0 {
        return Ok(Json(VoteResponse {
            success: false,
            message: "Vote not found".to_string(),
            vote_id: None,
            data: None,
        }));
    }

    println!("✅ Vote deleted successfully");
    Ok(Json(VoteResponse {
        success: true,
        message: "Vote deleted successfully".to_string(),
        vote_id: Some(vote_id),
        data: None,
    }))
}

// ========== LIKE HANDLERS ==========

pub async fn create_like(
    State(state): State<AppState>,
    Json(payload): Json<CreateLike>,
) -> Result<Json<LikeResponse>> {
    println!("👍 Creating like for user: {} ({})", payload.username, payload.voter_id);

    payload.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    let collection: Collection<Like> = state.db.collection("likes");
    let existing_like_filter = doc! {
        "voterId": &payload.voter_id,
        "fixtureId": &payload.fixture_id,
    };

    let existing_like = collection.find_one(existing_like_filter.clone()).await?;
    let total_likes: i64;
    let message: String;
    let success: bool;
    let mut like_id: Option<String> = None;

    if let Some(_like) = existing_like {
        if payload.action == "unlike" {
            collection.delete_one(existing_like_filter).await?;
            let fixture_filter = doc! { "fixtureId": &payload.fixture_id };
            total_likes = collection.count_documents(fixture_filter).await? as i64;
            message = "Like removed successfully".to_string();
            success = true;
            println!("👎 Like removed for fixture: {} by {}", payload.fixture_id, payload.username);
        } else {
            return Ok(Json(LikeResponse {
                success: false,
                message: "User already liked this fixture".to_string(),
                like_id: None,
                total_likes: 0,
            }));
        }
    } else {
        if payload.action != "like" {
            return Ok(Json(LikeResponse {
                success: false,
                message: "Cannot unlike a fixture you haven't liked".to_string(),
                like_id: None,
                total_likes: 0,
            }));
        }

        let like = Like {
            id: None,
            voter_id: payload.voter_id.clone(),
            username: payload.username.clone(),
            fixture_id: payload.fixture_id.clone(),
            action: payload.action.clone(),
            like_timestamp: BsonDateTime::from_chrono(Utc::now()),
            created_at: Some(BsonDateTime::from_chrono(Utc::now())),
        };

        let insert_result = collection.insert_one(like).await?;
        let fixture_filter = doc! { "fixtureId": &payload.fixture_id };
        total_likes = collection.count_documents(fixture_filter).await? as i64;
        message = "Like added successfully".to_string();
        success = true;

        // Get the inserted ID if needed
        if let Some(id) = insert_result.inserted_id.as_object_id() {
            like_id = Some(id.to_hex());
        }

        println!("✅ Like created for fixture: {} by {}", payload.fixture_id, payload.username);

        // ===== ADD LIKE NOTIFICATIONS HERE =====
        let state_clone = state.clone();
        let payload_clone = payload.clone();

        tokio::spawn(async move {
            // Initialize FCM service
            if let Ok(fcm_service) = crate::services::fcm_service::init_fcm_service().await {

                // Get fixture details from games collection
                let games_collection: Collection<Game> = state_clone.db.collection("games");
                let game_filter = doc! { "fixtureId": &payload_clone.fixture_id };

                let (home_team, away_team) = match games_collection.find_one(game_filter).await {
                    Ok(Some(game)) => (game.home_team.clone(), game.away_team.clone()),
                    _ => ("Unknown".to_string(), "Unknown".to_string()),
                };

                let fixture_name = format!("{} vs {}", home_team, away_team);

                // Get all users who have interacted with this fixture
                let mut user_ids = Vec::new();

                // Get voters
                let vote_collection: Collection<Vote> = state_clone.db.collection("votes");
                let vote_filter = doc! { "fixture_id": &payload_clone.fixture_id };
                if let Ok(cursor) = vote_collection.find(vote_filter).await {
                    let votes: Vec<Vote> = cursor.try_collect().await.unwrap_or_default();
                    for vote in votes {
                        if vote.voter_id != payload_clone.voter_id {
                            user_ids.push(vote.voter_id);
                        }
                    }
                }

                // Get commenters
                let comment_collection: Collection<Comment> = state_clone.db.collection("room");
                let comment_filter = doc! { "fixture_id": &payload_clone.fixture_id };
                if let Ok(cursor) = comment_collection.find(comment_filter).await {
                    let comments: Vec<Comment> = cursor.try_collect().await.unwrap_or_default();
                    for comment in comments {
                        if comment.voter_id != payload_clone.voter_id {
                            user_ids.push(comment.voter_id);
                        }
                    }
                }

                // Get other users who liked this fixture
                let like_collection: Collection<Like> = state_clone.db.collection("likes");
                let other_likes_filter = doc! {
                    "fixture_id": &payload_clone.fixture_id,
                    "voterId": { "$ne": &payload_clone.voter_id }
                };
                if let Ok(cursor) = like_collection.find(other_likes_filter).await {
                    let likes: Vec<Like> = cursor.try_collect().await.unwrap_or_default();
                    for like in likes {
                        user_ids.push(like.voter_id);
                    }
                }

                // Remove duplicates
                user_ids.sort();
                user_ids.dedup();

                if !user_ids.is_empty() {
                    println!("📱 Notifying {} users about new like", user_ids.len());

                    // Get current like count
                    let like_count = user_ids.len() as i64 + 1; // +1 for current user

                    let _ = fcm_service.send_to_multiple_users(
                        &state_clone,
                        user_ids,
                        "❤️ Someone liked this match",
                        &format!("@{} liked {} ({} likes total)",
                            payload_clone.username,
                            fixture_name,
                            like_count
                        ),
                        serde_json::json!({
                            "fixture_id": payload_clone.fixture_id,
                            "liker_id": payload_clone.voter_id,
                            "liker_username": payload_clone.username,
                            "like_count": like_count,
                            "home_team": home_team,
                            "away_team": away_team,
                            "type": "like_notification",
                            "action": "new_like"
                        }),
                        "like_notification"
                    ).await;
                }
            }
        });
        // ===== END LIKE NOTIFICATIONS =====
    }

    // Return the response
    Ok(Json(LikeResponse {
        success,
        message,
        like_id,
        total_likes,
    }))
}

pub async fn get_fixture_likes(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<LikeStats>> {
    println!("👍 Getting likes for fixture: {}", fixture_id);

    let collection: Collection<Like> = state.db.collection("likes");

    // Get total likes for fixture
    let filter = doc! { "fixture_id": &fixture_id };
    let total_likes = collection.count_documents(filter).await? as i64;

    let stats = LikeStats {
        fixture_id: fixture_id.clone(),
        total_likes,
        user_has_liked: false, // This would need user context
    };

    println!("✅ Found {} likes for fixture", total_likes);
    Ok(Json(stats))
}

pub async fn get_total_likes_for_fixture(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    println!("👍 Getting total like count for fixture: {}", fixture_id);

    let collection: Collection<Like> = state.db.collection("likes");
    let filter = doc! { "fixture_id": &fixture_id };

    let total_likes = collection.count_documents(filter).await? as i64;

    let response = json!({
        "success": true,
        "fixture_id": fixture_id,
        "total_likes": total_likes,
        "timestamp": Utc::now().to_rfc3339(),
    });

    println!("✅ Total likes for fixture {}: {}", fixture_id, total_likes);
    Ok(Json(response))
}

pub async fn get_user_like_for_fixture(
    State(state): State<AppState>,
    Path((fixture_id, voter_id)): Path<(String, String)>,
) -> Result<Json<LikeStats>> {
    println!(
        "👍 Checking user like: {} for fixture: {}",
        voter_id, fixture_id
    );

    let collection: Collection<Like> = state.db.collection("likes");

    // Get total likes for fixture
    let fixture_filter = doc! { "fixture_id": &fixture_id };
    let total_likes = collection.count_documents(fixture_filter).await? as i64;

    // Check if user has liked
    let user_like_filter = doc! {
        "fixture_id": &fixture_id,
        "voterId": &voter_id,
    };
    let user_has_liked = collection.find_one(user_like_filter).await?.is_some();

    let stats = LikeStats {
        fixture_id: fixture_id.clone(),
        total_likes,
        user_has_liked,
    };

    println!("✅ User {} has liked: {}", voter_id, user_has_liked);
    Ok(Json(stats))
}

pub async fn delete_like(
    State(state): State<AppState>,
    Path(like_id): Path<String>,
) -> Result<Json<LikeResponse>> {
    println!("🗑️ Deleting like: {}", like_id);

    let collection: Collection<Like> = state.db.collection("likes");

    let object_id = ObjectId::parse_str(&like_id)
        .map_err(|_| AppError::invalid_data("Invalid like ID format"))?;

    let filter = doc! { "_id": object_id };

    // Get the like before deleting to get fixture_id
    let like = collection
        .find_one(filter.clone())
        .await?
        .ok_or_else(|| AppError::DocumentNotFound)?;

    let delete_result = collection.delete_one(filter).await?;

    if delete_result.deleted_count == 0 {
        return Ok(Json(LikeResponse {
            success: false,
            message: "Like not found".to_string(),
            like_id: None,
            total_likes: 0,
        }));
    }

    // Get updated like count for the fixture
    let fixture_filter = doc! { "fixture_id": &like.fixture_id };
    let total_likes = collection.count_documents(fixture_filter).await? as i64;

    println!("✅ Like deleted successfully");
    Ok(Json(LikeResponse {
        success: true,
        message: "Like deleted successfully".to_string(),
        like_id: Some(like_id),
        total_likes,
    }))
}

// ========== COMMENT HANDLERS (UPDATED WITH SELECTION) ==========

pub async fn create_comment(
    State(state): State<AppState>,
    Json(payload): Json<CreateComment>,
) -> Result<Json<CommentResponse>> {
    println!("💬 Creating comment for user: {} ({})", payload.username, payload.voter_id);

    // Validate payload
    payload
        .validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Validate selection field is provided
    if payload.selection.trim().is_empty() {
        return Err(AppError::ValidationError(
            "selection is required".to_string(),
        ));
    }

    // Validate selection is valid
    validate_selection(&payload.selection).map_err(|e| AppError::ValidationError(e))?;

    // Validate fixture_id is provided
    if payload.fixture_id.trim().is_empty() {
        return Err(AppError::ValidationError(
            "fixtureId is required".to_string(),
        ));
    }

    let collection: Collection<Comment> = state.db.collection("room");

    // Parse the timestamp from Flutter using helper function
    let comment_timestamp = parse_iso_timestamp_or_now(&payload.timestamp);

    let comment = Comment {
        id: None,
        voter_id: payload.voter_id.clone(),
        username: payload.username.clone(),
        fixture_id: payload.fixture_id.clone(),
        selection: payload.selection.clone(), // NEW: Store user's vote selection
        comment: payload.comment.clone(),
        timestamp: payload.timestamp.clone(),
        comment_timestamp,
        created_at: Some(BsonDateTime::from_chrono(Utc::now())),
        likes: Some(0),
        replies: Some(Vec::new()),
    };

    let insert_result = collection.insert_one(comment).await?;
    let comment_id = insert_result.inserted_id.as_object_id().unwrap().to_hex();

    // CLONE the comment_id for use in the closure
    let comment_id_for_closure = comment_id.clone();

    // Fetch the inserted comment
    let filter = doc! { "_id": insert_result.inserted_id };
    let inserted_comment = collection
        .find_one(filter)
        .await?
        .ok_or_else(|| AppError::DocumentNotFound)?;

    println!("✅ Comment created successfully: {} by {}", comment_id, payload.username);

    // ===== SEND FCM NOTIFICATIONS FOR COMMENT =====
    let state_clone = state.clone();
    let payload_clone = payload.clone();
    let comment_text = payload.comment.clone();

    tokio::spawn(async move {
        // Initialize FCM service
        if let Ok(fcm_service) = crate::services::fcm_service::init_fcm_service().await {

            // Get fixture details from games collection
            let games_collection: Collection<Game> = state_clone.db.collection("games");
            let game_filter = doc! { "fixtureId": &payload_clone.fixture_id };

            let (home_team, away_team) = match games_collection.find_one(game_filter).await {
                Ok(Some(game)) => (game.home_team.clone(), game.away_team.clone()),
                _ => ("Unknown".to_string(), "Unknown".to_string()),
            };

            let fixture_name = format!("{} vs {}", home_team, away_team);

            // Get all users who have interacted with this fixture
            let mut user_ids = Vec::new();

            // Get voters
            let vote_collection: Collection<Vote> = state_clone.db.collection("votes");
            let vote_filter = doc! { "fixture_id": &payload_clone.fixture_id };
            if let Ok(cursor) = vote_collection.find(vote_filter).await {
                let votes: Vec<Vote> = cursor.try_collect().await.unwrap_or_default();
                for vote in votes {
                    if vote.voter_id != payload_clone.voter_id {
                        user_ids.push(vote.voter_id);
                    }
                }
            }

            // Get other commenters
            let comment_collection: Collection<Comment> = state_clone.db.collection("room");
            let comment_filter = doc! {
                "fixture_id": &payload_clone.fixture_id,
                "voterId": { "$ne": &payload_clone.voter_id }
            };
            if let Ok(cursor) = comment_collection.find(comment_filter).await {
                let comments: Vec<Comment> = cursor.try_collect().await.unwrap_or_default();
                for comment in comments {
                    if comment.voter_id != payload_clone.voter_id {
                        user_ids.push(comment.voter_id);
                    }
                }
            }

            // Get users who liked this fixture
            let like_collection: Collection<Like> = state_clone.db.collection("likes");
            let like_filter = doc! { "fixture_id": &payload_clone.fixture_id };
            if let Ok(cursor) = like_collection.find(like_filter).await {
                let likes: Vec<Like> = cursor.try_collect().await.unwrap_or_default();
                for like in likes {
                    if like.voter_id != payload_clone.voter_id {
                        user_ids.push(like.voter_id);
                    }
                }
            }

            // Remove duplicates
            user_ids.sort();
            user_ids.dedup();

            let short_comment = if comment_text.len() > 30 {
                format!("{}...", &comment_text[0..30])
            } else {
                comment_text.clone()
            };

            if !user_ids.is_empty() {
                println!("📱 Notifying {} users about new comment", user_ids.len());
                let _ = fcm_service.send_to_multiple_users(
                    &state_clone,
                    user_ids,
                    "💬 New comment on fixture",
                    &format!("{} commented: \"{}\" on {}",
                        payload_clone.username,
                        short_comment,
                        fixture_name
                    ),
                    serde_json::json!({
                        "fixture_id": payload_clone.fixture_id,
                        "comment_id": comment_id_for_closure,
                        "voter_id": payload_clone.voter_id,
                        "voter_username": payload_clone.username,
                        "voter_selection": payload_clone.selection, // Include selection in notification
                        "comment": comment_text,
                        "home_team": home_team,
                        "away_team": away_team,
                        "type": "comment_notification",
                        "action": "new_comment"
                    }),
                    "comment_notification"
                ).await;
            }
        }
    });

    Ok(Json(CommentResponse {
        success: true,
        message: "Comment submitted successfully".to_string(),
        comment_id: Some(comment_id),
        comment: Some(inserted_comment),
    }))
}

pub async fn get_comments(
    State(state): State<AppState>,
    Query(query): Query<CommentQuery>,
) -> Result<Json<Vec<Comment>>> {
    println!("🔍 Getting comments...");

    let collection: Collection<Comment> = state.db.collection("room");
    let mut filter = doc! {};

    if let Some(fixture_id) = &query.fixture_id {
        filter.insert("fixtureId", fixture_id);
    }

    if let Some(voter_id) = &query.voter_id {
        filter.insert("voterId", voter_id);
    }

    // NEW: Add selection filter if provided
    if let Some(selection) = &query.selection {
        filter.insert("selection", selection);
    }

    let _options = FindOptions::builder()
        .sort(doc! { "comment_timestamp": -1 })
        .build();

    let cursor = collection.find(filter).await?;
    let comments: Vec<Comment> = cursor.try_collect().await?;

    println!("✅ Found {} comments", comments.len());
    Ok(Json(comments))
}

pub async fn get_fixture_comments(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<CommentStats>> {
    println!("💬 Getting comments for fixture: {}", fixture_id);

    let collection: Collection<Comment> = state.db.collection("room");
    let filter = doc! { "fixture_id": &fixture_id };

    let _options = FindOptions::builder()
        .sort(doc! { "comment_timestamp": -1 })
        .limit(20)
        .build();

    let cursor = collection.find(filter).await?;
    let all_comments: Vec<Comment> = cursor.try_collect().await?;

    let total_comments = all_comments.len() as i64;

    // NEW: Get comment counts by selection
    let home_comments = all_comments.iter().filter(|c| c.selection == "home_team").count() as i64;
    let draw_comments = all_comments.iter().filter(|c| c.selection == "draw").count() as i64;
    let away_comments = all_comments.iter().filter(|c| c.selection == "away_team").count() as i64;

    // Get recent comments (already sorted by timestamp)
    let recent_comments: Vec<Comment> = all_comments.into_iter().take(10).collect();

    let stats = CommentStats {
        fixture_id: fixture_id.clone(),
        total_comments,
        home_comments, // NEW
        draw_comments, // NEW
        away_comments, // NEW
        recent_comments,
    };

    println!("✅ Found {} comments for fixture (H:{} D:{} A:{})",
        total_comments, home_comments, draw_comments, away_comments);
    Ok(Json(stats))
}

pub async fn get_total_comments_for_fixture(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    println!("💬 Getting total comment count for fixture: {}", fixture_id);

    let collection: Collection<Comment> = state.db.collection("room");
    let filter = doc! { "fixture_id": &fixture_id };

    let total_comments = collection.count_documents(filter).await? as i64;

    let response = json!({
        "success": true,
        "fixture_id": fixture_id,
        "total_comments": total_comments,
        "timestamp": Utc::now().to_rfc3339(),
    });

    println!("✅ Total comments for fixture {}: {}", fixture_id, total_comments);
    Ok(Json(response))
}

pub async fn get_user_comments(
    State(state): State<AppState>,
    Path(voter_id): Path<String>,
) -> Result<Json<Vec<Comment>>> {
    println!("🔍 Getting comments for user: {}", voter_id);

    let collection: Collection<Comment> = state.db.collection("room");
    let filter = doc! { "voterId": voter_id };

    let _options = FindOptions::builder()
        .sort(doc! { "comment_timestamp": -1 })
        .build();

    let cursor = collection.find(filter).await?;
    let comments: Vec<Comment> = cursor.try_collect().await?;

    println!("✅ Found {} comments for user", comments.len());
    Ok(Json(comments))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    Path(comment_id): Path<String>,
) -> Result<Json<CommentResponse>> {
    println!("🗑️ Deleting comment: {}", comment_id);

    let collection: Collection<Comment> = state.db.collection("room");

    let object_id = ObjectId::parse_str(&comment_id)
        .map_err(|_| AppError::invalid_data("Invalid comment ID format"))?;

    let filter = doc! { "_id": object_id };

    let delete_result = collection.delete_one(filter).await?;

    if delete_result.deleted_count == 0 {
        return Ok(Json(CommentResponse {
            success: false,
            message: "Comment not found".to_string(),
            comment_id: None,
            comment: None,
        }));
    }

    println!("✅ Comment deleted successfully");
    Ok(Json(CommentResponse {
        success: true,
        message: "Comment deleted successfully".to_string(),
        comment_id: Some(comment_id),
        comment: None,
    }))
}

pub async fn like_comment(
    State(state): State<AppState>,
    Path(comment_id): Path<String>,
) -> Result<Json<CommentResponse>> {
    println!("👍 Liking comment: {}", comment_id);

    let collection: Collection<Comment> = state.db.collection("room");

    let object_id = ObjectId::parse_str(&comment_id)
        .map_err(|_| AppError::invalid_data("Invalid comment ID format"))?;

    let filter = doc! { "_id": object_id };
    let update = doc! { "$inc": { "likes": 1 } };

    let update_result = collection.update_one(filter, update).await?;

    if update_result.matched_count == 0 {
        return Ok(Json(CommentResponse {
            success: false,
            message: "Comment not found".to_string(),
            comment_id: None,
            comment: None,
        }));
    }

    // Fetch updated comment
    let updated_filter = doc! { "_id": object_id };
    let updated_comment = collection
        .find_one(updated_filter)
        .await?
        .ok_or_else(|| AppError::DocumentNotFound)?;

    println!("✅ Comment liked successfully");
    Ok(Json(CommentResponse {
        success: true,
        message: "Comment liked successfully".to_string(),
        comment_id: Some(comment_id),
        comment: Some(updated_comment),
    }))
}

// ========== STATISTICS HANDLERS ==========

pub async fn get_vote_stats(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<VoteStats>> {
    get_fixture_votes(State(state), Path(fixture_id)).await
}

pub async fn get_like_stats(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<LikeStats>> {
    get_fixture_likes(State(state), Path(fixture_id)).await
}

pub async fn get_comment_stats(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<CommentStats>> {
    get_fixture_comments(State(state), Path(fixture_id)).await
}

pub async fn get_fixture_stats(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<FixtureStats>> {
    println!("📊 Getting comprehensive stats for fixture: {}", fixture_id);

    // Get vote stats
    let vote_stats = get_vote_stats(State(state.clone()), Path(fixture_id.clone()))
        .await?
        .0;

    // Get like stats (without user context)
    let like_stats = get_like_stats(State(state.clone()), Path(fixture_id.clone()))
        .await?
        .0;

    // Get comment stats
    let comment_stats = get_comment_stats(State(state.clone()), Path(fixture_id.clone()))
        .await?
        .0;

    let stats = FixtureStats {
        fixture_id: fixture_id.clone(),
        home_team: vote_stats.home_team.clone(),
        away_team: vote_stats.away_team.clone(),
        vote_stats,
        like_stats,
        comment_stats,
    };

    println!("✅ Comprehensive stats generated for fixture");
    Ok(Json(stats))
}

pub async fn get_all_counts_for_fixture(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<FixtureCountsResponse>> {
    println!("📊 Getting all counts for fixture: {}", fixture_id);

    let vote_collection: Collection<Vote> = state.db.collection("votes");
    let like_collection: Collection<Like> = state.db.collection("likes");
    let comment_collection: Collection<Comment> = state.db.collection("comments");

    // Get vote counts
    let vote_filter = doc! { "fixture_id": &fixture_id };
    let total_votes = vote_collection.count_documents(vote_filter.clone()).await? as i64;

    let home_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "home_team"
        })
        .await? as i64;

    let draw_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "draw"
        })
        .await? as i64;

    let away_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "away_team"
        })
        .await? as i64;

    // Get like count
    let like_filter = doc! { "fixture_id": &fixture_id };
    let total_likes = like_collection.count_documents(like_filter).await? as i64;

    // Get comment count
    let comment_filter = doc! { "fixture_id": &fixture_id };
    let total_comments = comment_collection.count_documents(comment_filter).await? as i64;

    // Get fixture details from first vote (if exists)
    let first_vote = vote_collection.find_one(vote_filter).await?;
    let (home_team, away_team) = if let Some(vote) = first_vote {
        (vote.home_team.clone(), vote.away_team.clone())
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
    };

    let total_engagement = total_votes + total_likes + total_comments;

    let counts = crate::models::vote::FixtureCounts {
        fixture_id: fixture_id.clone(),
        home_team,
        away_team,
        total_votes,
        home_votes,
        draw_votes,
        away_votes,
        total_likes,
        total_comments,
        total_engagement,
        user_has_voted: false,
        user_has_liked: false,
        user_selection: None,
    };

    let response = FixtureCountsResponse {
        success: true,
        message: format!("Counts retrieved for fixture {}", fixture_id),
        data: counts,
    };

    println!("✅ All counts for fixture {}: {} votes, {} likes, {} comments",
        fixture_id, total_votes, total_likes, total_comments);
    Ok(Json(response))
}

pub async fn get_user_stats(
    State(state): State<AppState>,
    Path(voter_id): Path<String>,
) -> Result<Json<UserVoteStatus>> {
    println!("👤 Getting stats for user: {}", voter_id);

    // Get user's votes
    let vote_collection: Collection<Vote> = state.db.collection("votes");
    let vote_filter = doc! { "voterId": &voter_id };
    let votes_count = vote_collection.count_documents(vote_filter).await? as i64;

    // Get user's likes
    let like_collection: Collection<Like> = state.db.collection("likes");
    let like_filter = doc! { "voterId": &voter_id };
    let likes_count = like_collection.count_documents(like_filter).await? as i64;

    // Get user's comments
    let comment_collection: Collection<Comment> = state.db.collection("comments");
    let comment_filter = doc! { "voterId": &voter_id };
    let comments_count = comment_collection.count_documents(comment_filter).await? as i64;

    let stats = UserVoteStatus {
        fixture_id: "all".to_string(), // For overall user stats
        has_voted: votes_count > 0,
        vote_selection: None, // Can't determine for all fixtures
        has_liked: likes_count > 0,
        user_comments_count: comments_count,
    };

    println!(
        "✅ User stats: {} votes, {} likes, {} comments",
        votes_count, likes_count, comments_count
    );
    Ok(Json(stats))
}

pub async fn get_total_counts(State(state): State<AppState>) -> Result<Json<TotalCountsResponse>> {
    println!("📈 Getting total counts across all fixtures");

    let vote_collection: Collection<Vote> = state.db.collection("votes");
    let like_collection: Collection<Like> = state.db.collection("likes");
    let comment_collection: Collection<Comment> = state.db.collection("comments");

    // Get total counts
    let total_votes = vote_collection.estimated_document_count().await? as i64;
    let total_likes = like_collection.estimated_document_count().await? as i64;
    let total_comments = comment_collection.estimated_document_count().await? as i64;

    // Get unique users (distinct voterIds)
    let unique_users = vote_collection.distinct("voterId", doc! {}).await?.len() as i64;
    let total_engagement = total_votes + total_likes + total_comments;

    let counts = crate::models::vote::TotalCounts {
        total_votes,
        total_likes,
        total_comments,
        total_engagement,
        total_users: unique_users,
        timestamp: Utc::now().to_rfc3339(),
    };

    let response = TotalCountsResponse {
        success: true,
        message: "Total counts retrieved successfully".to_string(),
        data: counts,
    };

    println!("✅ Total counts: {} votes, {} likes, {} comments, {} users",
        total_votes, total_likes, total_comments, unique_users);
    Ok(Json(response))
}

pub async fn get_batch_fixture_counts(
    State(state): State<AppState>,
    Json(payload): Json<crate::models::vote::BatchFixtureCountsRequest>,
) -> Result<Json<crate::models::vote::BatchFixtureCountsResponse>> {
    println!("📊 Getting batch counts for {} fixtures", payload.fixture_ids.len());

    let vote_collection: Collection<Vote> = state.db.collection("votes");
    let like_collection: Collection<Like> = state.db.collection("likes");
    let comment_collection: Collection<Comment> = state.db.collection("comments");

    let mut fixture_counts = Vec::new();

    for fixture_id in payload.fixture_ids {
        // Get vote counts
        let vote_filter = doc! { "fixture_id": &fixture_id };
        let total_votes = vote_collection.count_documents(vote_filter.clone()).await? as i64;

        // Get vote breakdown
        let home_votes = vote_collection
            .count_documents(doc! {
                "fixture_id": &fixture_id,
                "selection": "home_team"
            })
            .await? as i64;

        let draw_votes = vote_collection
            .count_documents(doc! {
                "fixture_id": &fixture_id,
                "selection": "draw"
            })
            .await? as i64;

        let away_votes = vote_collection
            .count_documents(doc! {
                "fixture_id": &fixture_id,
                "selection": "away_team"
            })
            .await? as i64;

        // Get like count
        let like_filter = doc! { "fixture_id": &fixture_id };
        let total_likes = like_collection.count_documents(like_filter).await? as i64;

        // Get comment count
        let comment_filter = doc! { "fixture_id": &fixture_id };
        let total_comments = comment_collection.count_documents(comment_filter).await? as i64;

        // Get fixture details from first vote (if exists)
        let first_vote = vote_collection.find_one(vote_filter).await?;
        let (home_team, away_team) = if let Some(vote) = first_vote {
            (vote.home_team.clone(), vote.away_team.clone())
        } else {
            ("Unknown".to_string(), "Unknown".to_string())
        };

        let total_engagement = total_votes + total_likes + total_comments;

        // Check if user has voted/liked (if user_id is provided)
        let mut user_has_voted = None;
        let mut user_has_liked = None;
        let mut user_selection = None;

        if let Some(user_id) = &payload.user_id {
            // Check if user voted
            let user_vote_filter = doc! {
                "fixture_id": &fixture_id,
                "voterId": user_id,
            };
            let user_vote = vote_collection.find_one(user_vote_filter).await?;
            user_has_voted = Some(user_vote.is_some());
            user_selection = user_vote.map(|v| v.selection);

            // Check if user liked
            let user_like_filter = doc! {
                "fixture_id": &fixture_id,
                "voterId": user_id,
            };
            let user_like = like_collection.find_one(user_like_filter).await?;
            user_has_liked = Some(user_like.is_some());
        }

        let count_item = crate::models::vote::FixtureCountItem {
            fixture_id: fixture_id.clone(),
            home_team,
            away_team,
            total_votes,
            total_likes,
            total_comments,
            total_engagement,
            user_has_voted,
            user_has_liked,
            user_selection,
        };

        fixture_counts.push(count_item);
    }

    // Get the count before moving the vector
    let count = fixture_counts.len();

    let response = crate::models::vote::BatchFixtureCountsResponse {
        success: true,
        message: format!("Counts retrieved for {} fixtures", count),
        data: fixture_counts,
        count,
    };

    println!("✅ Batch counts retrieved for {} fixtures", count);
    Ok(Json(response))
}

// ========== ADMIN HANDLERS ==========

pub async fn cleanup_old_votes(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    println!("🧹 Cleaning up old votes...");

    let collection: Collection<Vote> = state.db.collection("votes");

    // Delete votes older than 30 days
    let cutoff_date = Utc::now() - Duration::days(30);
    let cutoff_bson = BsonDateTime::from_chrono(cutoff_date);

    let filter = doc! {
        "vote_timestamp": {
            "$lt": cutoff_bson
        }
    };

    let delete_result = collection.delete_many(filter).await?;

    let response = json!({
        "success": true,
        "message": format!("Cleaned up {} old votes", delete_result.deleted_count),
        "deleted_count": delete_result.deleted_count,
        "timestamp": Utc::now().to_rfc3339(),
    });

    println!(
        "✅ Cleanup completed: {} votes deleted",
        delete_result.deleted_count
    );
    Ok(Json(response))
}

pub async fn get_overview_stats(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    println!("📈 Getting overview statistics...");

    // Get counts from all collections
    let vote_collection: Collection<Vote> = state.db.collection("votes");
    let like_collection: Collection<Like> = state.db.collection("likes");
    let comment_collection: Collection<Comment> = state.db.collection("comments");

    let total_votes = vote_collection.estimated_document_count().await? as i64;
    let total_likes = like_collection.estimated_document_count().await? as i64;
    let total_comments = comment_collection.estimated_document_count().await? as i64;

    // Get votes by selection
    let home_votes = vote_collection
        .count_documents(doc! { "selection": "home_team" })
        .await? as i64;
    let draw_votes = vote_collection
        .count_documents(doc! { "selection": "draw" })
        .await? as i64;
    let away_votes = vote_collection
        .count_documents(doc! { "selection": "away_team" })
        .await? as i64;

    // Get comments by selection (NEW)
    let home_comments = comment_collection
        .count_documents(doc! { "selection": "home_team" })
        .await? as i64;
    let draw_comments = comment_collection
        .count_documents(doc! { "selection": "draw" })
        .await? as i64;
    let away_comments = comment_collection
        .count_documents(doc! { "selection": "away_team" })
        .await? as i64;

    // Get today's votes
    let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_end = Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap();

    let today_start_bson = BsonDateTime::from_chrono(today_start.and_utc());
    let today_end_bson = BsonDateTime::from_chrono(today_end.and_utc());

    let today_votes = vote_collection
        .count_documents(doc! {
            "vote_timestamp": {
                "$gte": today_start_bson,
                "$lte": today_end_bson
            }
        })
        .await? as i64;

    let stats = json!({
        "success": true,
        "data": {
            "totals": {
                "votes": total_votes,
                "likes": total_likes,
                "comments": total_comments
            },
            "vote_distribution": {
                "home_team": home_votes,
                "draw": draw_votes,
                "away_team": away_votes
            },
            "comment_distribution": { // NEW
                "home_team": home_comments,
                "draw": draw_comments,
                "away_team": away_comments
            },
            "today": {
                "votes": today_votes
            },
            "percentages": {
                "home": if total_votes > 0 { (home_votes as f64 / total_votes as f64) * 100.0 } else { 0.0 },
                "draw": if total_votes > 0 { (draw_votes as f64 / total_votes as f64) * 100.0 } else { 0.0 },
                "away": if total_votes > 0 { (away_votes as f64 / total_votes as f64) * 100.0 } else { 0.0 }
            }
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    println!("✅ Overview stats generated");
    Ok(Json(stats))
}

// ========== ADDITIONAL HANDLERS FOR COMMENT COUNTS AND TOTAL LIKES ==========

pub async fn get_comment_counts_for_multiple_fixtures(
    State(state): State<AppState>,
    Json(fixture_ids): Json<Vec<String>>,
) -> Result<Json<serde_json::Value>> {
    println!(
        "📊 Getting comment counts for {} fixtures",
        fixture_ids.len()
    );

    let collection: Collection<Comment> = state.db.collection("room");

    let mut result = serde_json::Map::new();

    for fixture_id in fixture_ids {
        let filter = doc! { "fixture_id": &fixture_id };
        let count = collection.count_documents(filter).await? as i64;
        result.insert(fixture_id, serde_json::Value::Number(count.into()));
    }

    println!("✅ Comment counts retrieved for all fixtures");
    Ok(Json(serde_json::Value::Object(result)))
}

pub async fn get_total_likes_for_multiple_fixtures(
    State(state): State<AppState>,
    Json(fixture_ids): Json<Vec<String>>,
) -> Result<Json<serde_json::Value>> {
    println!("👍 Getting like counts for {} fixtures", fixture_ids.len());

    let collection: Collection<Like> = state.db.collection("likes");

    let mut result = serde_json::Map::new();

    for fixture_id in fixture_ids {
        let filter = doc! { "fixture_id": &fixture_id };
        let count = collection.count_documents(filter).await? as i64;
        result.insert(fixture_id, serde_json::Value::Number(count.into()));
    }

    println!("✅ Like counts retrieved for all fixtures");
    Ok(Json(serde_json::Value::Object(result)))
}

pub async fn get_combined_stats_for_multiple_fixtures(
    State(state): State<AppState>,
    Json(fixture_ids): Json<Vec<String>>,
) -> Result<Json<serde_json::Value>> {
    println!(
        "📈 Getting combined stats for {} fixtures",
        fixture_ids.len()
    );

    let mut result = Vec::new();

    for fixture_id in fixture_ids {
        // Get vote stats
        let vote_stats = get_vote_stats(State(state.clone()), Path(fixture_id.clone())).await?;

        // Get like stats
        let like_stats = get_like_stats(State(state.clone()), Path(fixture_id.clone())).await?;

        // Get comment stats (now includes selection breakdown)
        let comment_stats = get_comment_stats(State(state.clone()), Path(fixture_id.clone())).await?;

        let stats = json!({
            "fixture_id": fixture_id,
            "vote_stats": vote_stats.0,
            "like_stats": like_stats.0,
            "comment_stats": comment_stats.0, // Includes home/draw/away comment counts
        });

        result.push(stats);
    }

    println!("✅ Combined stats retrieved for all fixtures");
    Ok(Json(json!({
        "success": true,
        "data": result,
        "count": result.len(),
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

// ========== NEW HANDLERS FOR REAL-TIME UPDATES ==========

pub async fn get_realtime_vote_updates(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    println!(
        "🔄 Getting real-time vote updates for fixture: {}",
        fixture_id
    );

    let vote_collection: Collection<Vote> = state.db.collection("votes");
    let like_collection: Collection<Like> = state.db.collection("likes");
    let comment_collection: Collection<Comment> = state.db.collection("comments");

    // Get vote counts by selection
    let home_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "home_team"
        })
        .await? as i64;

    let draw_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "draw"
        })
        .await? as i64;

    let away_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "away_team"
        })
        .await? as i64;

    // Get like count
    let like_count = like_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id
        })
        .await? as i64;

    // Get comment count and breakdown by selection (NEW)
    let home_comments = comment_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "home_team"
        })
        .await? as i64;

    let draw_comments = comment_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "draw"
        })
        .await? as i64;

    let away_comments = comment_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "away_team"
        })
        .await? as i64;

    let comment_count = home_comments + draw_comments + away_comments;

    let response = json!({
        "success": true,
        "data": {
            "fixture_id": fixture_id,
            "votes": {
                "home": home_votes,
                "draw": draw_votes,
                "away": away_votes,
                "total": home_votes + draw_votes + away_votes
            },
            "likes": like_count,
            "comments": {
                "total": comment_count,
                "by_selection": { // NEW
                    "home_team": home_comments,
                    "draw": draw_comments,
                    "away_team": away_comments
                }
            },
            "total_engagement": (home_votes + draw_votes + away_votes) + like_count + comment_count,
            "last_updated": Utc::now().to_rfc3339()
        }
    });

    println!("✅ Real-time stats retrieved");
    Ok(Json(response))
}

pub async fn get_vote_counts_by_selection(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    println!("📊 Getting vote counts by selection for fixture: {}", fixture_id);

    let collection: Collection<Vote> = state.db.collection("votes");

    let home_votes = collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "home_team"
        })
        .await? as i64;

    let draw_votes = collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "draw"
        })
        .await? as i64;

    let away_votes = collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "away_team"
        })
        .await? as i64;

    let total_votes = home_votes + draw_votes + away_votes;

    let response = json!({
        "success": true,
        "fixture_id": fixture_id,
        "vote_counts": {
            "home_team": home_votes,
            "draw": draw_votes,
            "away_team": away_votes,
            "total": total_votes
        },
        "percentages": {
            "home": if total_votes > 0 { (home_votes as f64 / total_votes as f64) * 100.0 } else { 0.0 },
            "draw": if total_votes > 0 { (draw_votes as f64 / total_votes as f64) * 100.0 } else { 0.0 },
            "away": if total_votes > 0 { (away_votes as f64 / total_votes as f64) * 100.0 } else { 0.0 }
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    println!("✅ Vote counts by selection for fixture {}: H:{} D:{} A:{}",
        fixture_id, home_votes, draw_votes, away_votes);
    Ok(Json(response))
}

pub async fn get_comment_counts_by_selection(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    println!("📊 Getting comment counts by selection for fixture: {}", fixture_id);

    let collection: Collection<Comment> = state.db.collection("room");

    let home_comments = collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "home_team"
        })
        .await? as i64;

    let draw_comments = collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "draw"
        })
        .await? as i64;

    let away_comments = collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "away_team"
        })
        .await? as i64;

    let total_comments = home_comments + draw_comments + away_comments;

    let response = json!({
        "success": true,
        "fixture_id": fixture_id,
        "comment_counts": {
            "home_team": home_comments,
            "draw": draw_comments,
            "away_team": away_comments,
            "total": total_comments
        },
        "percentages": {
            "home": if total_comments > 0 { (home_comments as f64 / total_comments as f64) * 100.0 } else { 0.0 },
            "draw": if total_comments > 0 { (draw_comments as f64 / total_comments as f64) * 100.0 } else { 0.0 },
            "away": if total_comments > 0 { (away_comments as f64 / total_comments as f64) * 100.0 } else { 0.0 }
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    println!("✅ Comment counts by selection for fixture {}: H:{} D:{} A:{}",
        fixture_id, home_comments, draw_comments, away_comments);
    Ok(Json(response))
}

pub async fn get_fixture_engagement_summary(
    State(state): State<AppState>,
    Path(fixture_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    println!("📊 Getting engagement summary for fixture: {}", fixture_id);

    let vote_collection: Collection<Vote> = state.db.collection("votes");
    let like_collection: Collection<Like> = state.db.collection("likes");
    let comment_collection: Collection<Comment> = state.db.collection("comments");

    // Get counts
    let vote_filter = doc! { "fixture_id": &fixture_id };
    let total_votes = vote_collection.count_documents(vote_filter.clone()).await? as i64;
    let total_likes = like_collection.count_documents(vote_filter.clone()).await? as i64;
    let total_comments = comment_collection.count_documents(vote_filter.clone()).await? as i64;
    let total_engagement = total_votes + total_likes + total_comments;

    // Get breakdown by selection for votes and comments
    let home_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "home_team"
        })
        .await? as i64;

    let draw_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "draw"
        })
        .await? as i64;

    let away_votes = vote_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "away_team"
        })
        .await? as i64;

    let home_comments = comment_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "home_team"
        })
        .await? as i64;

    let draw_comments = comment_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "draw"
        })
        .await? as i64;

    let away_comments = comment_collection
        .count_documents(doc! {
            "fixture_id": &fixture_id,
            "selection": "away_team"
        })
        .await? as i64;

    // Get fixture details
    let first_vote = vote_collection.find_one(vote_filter).await?;
    let (home_team, away_team) = if let Some(vote) = first_vote {
        (vote.home_team.clone(), vote.away_team.clone())
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
    };

    // Calculate engagement score (weighted)
    let engagement_score = (total_votes as f64 * 1.0) +
                          (total_likes as f64 * 0.5) +
                          (total_comments as f64 * 1.5);

    let response = json!({
        "success": true,
        "fixture_id": fixture_id,
        "home_team": home_team,
        "away_team": away_team,
        "engagement_metrics": {
            "votes": {
                "total": total_votes,
                "by_selection": {
                    "home_team": home_votes,
                    "draw": draw_votes,
                    "away_team": away_votes
                }
            },
            "likes": total_likes,
            "comments": {
                "total": total_comments,
                "by_selection": {
                    "home_team": home_comments,
                    "draw": draw_comments,
                    "away_team": away_comments
                }
            },
            "total_engagement": total_engagement,
            "engagement_score": engagement_score
        },
        "engagement_percentages": {
            "vote_percentage": if total_engagement > 0 { (total_votes as f64 / total_engagement as f64) * 100.0 } else { 0.0 },
            "like_percentage": if total_engagement > 0 { (total_likes as f64 / total_engagement as f64) * 100.0 } else { 0.0 },
            "comment_percentage": if total_engagement > 0 { (total_comments as f64 / total_engagement as f64) * 100.0 } else { 0.0 }
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    println!("✅ Engagement summary for fixture {}: {} total engagement",
        fixture_id, total_engagement);
    Ok(Json(response))
}
