use bson::{oid::ObjectId, DateTime as BsonDateTime};
use chrono::{DateTime, NaiveDateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ========== VOTE MODELS ==========

// Vote model for storing votes
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Vote {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    #[serde(rename = "voterId")]
    #[validate(length(min = 1, message = "Voter ID is required"))]
    pub voter_id: String,

    #[serde(rename = "username")]
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[serde(rename = "fixtureId")]
    #[validate(length(min = 1, message = "Fixture ID is required"))]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    #[validate(length(min = 1, message = "Home team is required"))]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    #[validate(length(min = 1, message = "Away team is required"))]
    pub away_team: String,

    pub draw: String, // String literal "draw" as per requirement

    #[serde(rename = "selection")]
    #[validate(length(min = 1, message = "Selection is required"))]
    pub selection: String, // "home_team", "draw", or "away_team"

    #[serde(rename = "voteTimestamp")]
    pub vote_timestamp: BsonDateTime,

    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<BsonDateTime>,
}

// For creating new votes (from Flutter app)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateVote {
    #[serde(rename = "voterId")]
    #[validate(length(min = 1, message = "Voter ID is required"))]
    pub voter_id: String,

    #[serde(rename = "username")]
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[serde(rename = "fixtureId")]
    #[validate(length(min = 1, message = "Fixture ID is required"))]
    pub fixture_id: String,

    #[serde(rename = "awayTeam")]
    #[validate(length(min = 1, message = "Away team is required"))]
    pub away_team: String,

    #[serde(rename = "draw")]
    #[validate(length(min = 1, message = "Draw field is required"))]
    pub draw: String,

    #[serde(rename = "homeTeam")]
    #[validate(length(min = 1, message = "Home team is required"))]
    pub home_team: String,

    #[serde(rename = "selection")]
    #[validate(length(min = 1, message = "Selection is required"))]
    pub selection: String,
}

// ========== LIKE MODELS ==========

// Like model for storing likes
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Like {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    #[serde(rename = "voterId")]
    #[validate(length(min = 1, message = "Voter ID is required"))]
    pub voter_id: String,

    #[serde(rename = "username")]
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[serde(rename = "fixtureId")]
    #[validate(length(min = 1, message = "Fixture ID is required"))]
    pub fixture_id: String,

    #[serde(rename = "action")]
    #[validate(length(min = 1, message = "Action is required"))]
    pub action: String, // "like" or "unlike"

    #[serde(rename = "likeTimestamp")]
    pub like_timestamp: BsonDateTime,

    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<BsonDateTime>,
}

// For creating new likes (from Flutter app)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateLike {
    #[serde(rename = "voterId")]
    #[validate(length(min = 1, message = "Voter ID is required"))]
    pub voter_id: String,

    #[serde(rename = "username")]
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[serde(rename = "fixtureId")]
    #[validate(length(min = 1, message = "Fixture ID is required"))]
    pub fixture_id: String,

    #[serde(rename = "action")]
    #[validate(length(min = 1, message = "Action is required"))]
    pub action: String,
}

// ========== COMMENT MODELS (UPDATED WITH SELECTION) ==========

// Comment model for storing


#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Comment {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    #[serde(rename = "voterId")]
    #[validate(length(min = 1, message = "Voter ID is required"))]
    pub voter_id: String,

    #[serde(rename = "username")]
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[serde(rename = "fixtureId")]
    #[validate(length(min = 1, message = "Fixture ID is required"))]
    pub fixture_id: String,

    // NEW FIELD: User's vote selection when they commented
    #[serde(rename = "selection")]
    #[validate(length(min = 1, message = "Selection is required"))]
    pub selection: String, // "home_team", "draw", or "away_team"

    #[serde(rename = "comment")]
    #[validate(length(
        min = 1,
        max = 500,
        message = "Comment must be between 1 and 500 characters"
    ))]
    pub comment: String,

    #[serde(rename = "timestamp")]
    pub timestamp: String, // ISO 8601 string from Flutter

    #[serde(rename = "commentTimestamp")]
    pub comment_timestamp: BsonDateTime,

    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<BsonDateTime>,

    #[serde(rename = "likes", skip_serializing_if = "Option::is_none")]
    pub likes: Option<i32>,

    #[serde(rename = "replies", skip_serializing_if = "Option::is_none")]
    pub replies: Option<Vec<ObjectId>>,
}

// For creating new comments (from Flutter app) - UPDATED WITH SELECTION
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateComment {
    #[serde(rename = "voterId")]
    #[validate(length(min = 1, message = "Voter ID is required"))]
    pub voter_id: String,

    #[serde(rename = "username")]
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,

    #[serde(rename = "fixtureId")]
    #[validate(length(min = 1, message = "Fixture ID is required"))]
    pub fixture_id: String,

    // NEW FIELD: User's vote selection when they commented
    #[serde(rename = "selection")]
    #[validate(length(min = 1, message = "Selection is required"))]
    pub selection: String, // "home_team", "draw", or "away_team"

    #[serde(rename = "comment")]
    #[validate(length(
        min = 1,
        max = 500,
        message = "Comment must be between 1 and 500 characters"
    ))]
    pub comment: String,

    #[serde(rename = "timestamp")]
    #[validate(length(min = 1, message = "Timestamp is required"))]
    pub timestamp: String,
}

// ========== STATISTICS MODELS (UPDATED) ==========

// Vote statistics response
#[derive(Debug, Serialize, Deserialize)]
pub struct VoteStats {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,

    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(rename = "homeVotes")]
    pub home_votes: i64,

    #[serde(rename = "drawVotes")]
    pub draw_votes: i64,

    #[serde(rename = "awayVotes")]
    pub away_votes: i64,

    #[serde(rename = "homePercentage")]
    pub home_percentage: f64,

    #[serde(rename = "drawPercentage")]
    pub draw_percentage: f64,

    #[serde(rename = "awayPercentage")]
    pub away_percentage: f64,
}

// Like statistics response
#[derive(Debug, Serialize, Deserialize)]
pub struct LikeStats {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "totalLikes")]
    pub total_likes: i64,

    #[serde(rename = "userHasLiked")]
    pub user_has_liked: bool,
}

// Comment statistics response - UPDATED WITH SELECTION FIELDS
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentStats {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "totalComments")]
    pub total_comments: i64,

    // NEW: Comment counts by selection
    #[serde(rename = "homeComments")]
    pub home_comments: i64,

    #[serde(rename = "drawComments")]
    pub draw_comments: i64,

    #[serde(rename = "awayComments")]
    pub away_comments: i64,

    #[serde(rename = "recentComments")]
    pub recent_comments: Vec<Comment>,
}

// Comment with user info (for responses) - UPDATED WITH SELECTION
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentWithUser {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    #[serde(rename = "voterId")]
    pub voter_id: String,

    #[serde(rename = "username")]
    pub username: String,

    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    // NEW FIELD: User's vote selection
    #[serde(rename = "selection")]
    pub selection: String,

    pub comment: String,
    pub timestamp: String,

    #[serde(rename = "commentTimestamp")]
    pub comment_timestamp: BsonDateTime,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub likes: Option<i32>,

    #[serde(rename = "replies", skip_serializing_if = "Option::is_none")]
    pub replies: Option<Vec<ObjectId>>,

    #[serde(rename = "userDisplayName", skip_serializing_if = "Option::is_none")]
    pub user_display_name: Option<String>,

    #[serde(rename = "userAvatar", skip_serializing_if = "Option::is_none")]
    pub user_avatar: Option<String>,

    #[serde(rename = "isVerified")]
    pub is_verified: bool,
}

// User vote status
#[derive(Debug, Serialize, Deserialize)]
pub struct UserVoteStatus {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "hasVoted")]
    pub has_voted: bool,

    #[serde(rename = "voteSelection")]
    pub vote_selection: Option<String>,

    #[serde(rename = "hasLiked")]
    pub has_liked: bool,

    #[serde(rename = "userCommentsCount")]
    pub user_comments_count: i64,
}

// Combined stats for a fixture
#[derive(Debug, Serialize, Deserialize)]
pub struct FixtureStats {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,

    #[serde(rename = "voteStats")]
    pub vote_stats: VoteStats,

    #[serde(rename = "likeStats")]
    pub like_stats: LikeStats,

    #[serde(rename = "commentStats")]
    pub comment_stats: CommentStats, // Now includes selection breakdown
}

// ========== API RESPONSE WRAPPERS ==========

#[derive(Debug, Serialize)]
pub struct VoteResponse {
    pub success: bool,
    pub message: String,

    #[serde(rename = "voteId", skip_serializing_if = "Option::is_none")]
    pub vote_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vote>,
}

#[derive(Debug, Serialize)]
pub struct LikeResponse {
    pub success: bool,
    pub message: String,

    #[serde(rename = "likeId", skip_serializing_if = "Option::is_none")]
    pub like_id: Option<String>,

    #[serde(rename = "totalLikes")]
    pub total_likes: i64,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub success: bool,
    pub message: String,

    #[serde(rename = "commentId", skip_serializing_if = "Option::is_none")]
    pub comment_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<Comment>,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub success: bool,
    pub data: FixtureStats,
}

// ========== QUERY PARAMETERS (UPDATED) ==========

#[derive(Debug, Deserialize)]
pub struct VoteQuery {
    #[serde(rename = "fixtureId")]
    pub fixture_id: Option<String>,

    #[serde(rename = "voterId")]
    pub voter_id: Option<String>,
    pub limit: Option<i64>,
    pub skip: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CommentQuery {
    #[serde(rename = "fixtureId")]
    pub fixture_id: Option<String>,

    #[serde(rename = "voterId")]
    pub voter_id: Option<String>,

    // NEW: Filter by vote selection
    #[serde(rename = "selection")]
    pub selection: Option<String>,

    pub limit: Option<i64>,
    pub skip: Option<u64>,

    #[serde(rename = "sortBy")]
    pub sort_by: Option<String>, // "newest", "oldest", "most_liked"
}

// ========== BULK OPERATIONS ==========

#[derive(Debug, Deserialize)]
pub struct BulkVoteRequest {
    pub votes: Vec<CreateVote>,
}

#[derive(Debug, Serialize)]
pub struct BulkVoteResponse {
    pub success: bool,

    #[serde(rename = "insertedCount")]
    pub inserted_count: u64,

    #[serde(rename = "failedCount")]
    pub failed_count: u64,

    #[serde(rename = "failedVotes")]
    pub failed_votes: Vec<FailedVote>,
}

#[derive(Debug, Serialize)]
pub struct FailedVote {
    pub index: usize,
    pub error: String,

    #[serde(rename = "voteData")]
    pub vote_data: CreateVote,
}

// ========== VALIDATION HELPER ==========

// Validation for selection field
pub fn validate_selection(selection: &str) -> Result<(), String> {
    let valid_selections = vec!["home_team", "draw", "away_team"];
    if !valid_selections.contains(&selection) {
        return Err(format!("Selection must be one of: {:?}", valid_selections));
    }
    Ok(())
}

// ========== TIMESTAMP PARSING ==========

// Helper function for timestamp parsing
pub fn parse_iso_timestamp(timestamp_str: &str) -> Result<BsonDateTime, String> {
    // Log the timestamp we're trying to parse for debugging
    println!("🔍 Parsing timestamp: '{}'", timestamp_str);

    // Try RFC 3339 format first (most common)
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
        println!("✅ Parsed as RFC 3339: {}", dt);
        return Ok(BsonDateTime::from_millis(dt.timestamp_millis()));
    }

    // Try parsing as DateTime with Utc timezone (for strings ending with Z)
    if timestamp_str.ends_with('Z') {
        // Remove the Z and parse as naive datetime, then add UTC
        let without_z = timestamp_str.trim_end_matches('Z');

        // Try with milliseconds
        if let Ok(ndt) = NaiveDateTime::parse_from_str(without_z, "%Y-%m-%dT%H:%M:%S%.f") {
            let dt_utc = DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
            println!("✅ Parsed with milliseconds: {}", dt_utc);
            return Ok(BsonDateTime::from_millis(dt_utc.timestamp_millis()));
        }

        // Try without milliseconds
        if let Ok(ndt) = NaiveDateTime::parse_from_str(without_z, "%Y-%m-%dT%H:%M:%S") {
            let dt_utc = DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
            println!("✅ Parsed without milliseconds: {}", dt_utc);
            return Ok(BsonDateTime::from_millis(dt_utc.timestamp_millis()));
        }
    }

    // Try ISO 8601 format with explicit UTC offset (+00:00)
    if let Ok(dt) = DateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M:%S%.f%:z") {
        println!("✅ Parsed with timezone offset: {}", dt);
        return Ok(BsonDateTime::from_millis(dt.timestamp_millis()));
    }

    // Try without fractional seconds and with timezone offset
    if let Ok(dt) = DateTime::parse_from_str(timestamp_str, "%Y-%m-%dT%H:%M:%S%:z") {
        println!("✅ Parsed with timezone offset (no ms): {}", dt);
        return Ok(BsonDateTime::from_millis(dt.timestamp_millis()));
    }

    // Try as a simple date time string (space instead of T)
    let dt_clean = timestamp_str.replace('T', " ").replace('Z', "");

    // Try with milliseconds
    if let Ok(ndt) = NaiveDateTime::parse_from_str(&dt_clean, "%Y-%m-%d %H:%M:%S%.f") {
        let dt_utc = DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
        println!("✅ Parsed as space-separated with ms: {}", dt_utc);
        return Ok(BsonDateTime::from_millis(dt_utc.timestamp_millis()));
    }

    // Try without milliseconds
    if let Ok(ndt) = NaiveDateTime::parse_from_str(&dt_clean, "%Y-%m-%d %H:%M:%S") {
        let dt_utc = DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
        println!("✅ Parsed as space-separated: {}", dt_utc);
        return Ok(BsonDateTime::from_millis(dt_utc.timestamp_millis()));
    }

    // Try Unix timestamp (seconds since epoch)
    if let Ok(ts) = timestamp_str.parse::<i64>() {
        // Check if it's seconds (10 digits) or milliseconds (13 digits)
        if timestamp_str.len() == 10 {
            // Seconds timestamp
            if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                println!("✅ Parsed as Unix seconds: {}", dt);
                return Ok(BsonDateTime::from_millis(dt.timestamp_millis()));
            }
        } else if timestamp_str.len() >= 13 {
            // Milliseconds timestamp
            return Ok(BsonDateTime::from_millis(ts));
        }
    }

    // Try parsing as a date only (for fallback)
    if let Ok(ndt) = NaiveDate::parse_from_str(timestamp_str, "%Y-%m-%d") {
        let dt_utc = ndt.and_hms_opt(0, 0, 0).unwrap().and_utc();
        println!("⚠️ Parsed as date only: {}", dt_utc);
        return Ok(BsonDateTime::from_millis(dt_utc.timestamp_millis()));
    }

    // If all parsing fails, return error with helpful message
    println!("❌ Failed to parse timestamp: '{}'", timestamp_str);
    Err(format!(
        "Invalid timestamp format: '{}'. Expected ISO 8601 format (e.g., 2024-01-01T12:00:00Z)",
        timestamp_str
    ))
}

// Simple version that returns current time on failure
pub fn parse_iso_timestamp_or_now(timestamp_str: &str) -> BsonDateTime {
    parse_iso_timestamp(timestamp_str).unwrap_or_else(|_| {
        println!(
            "⚠️ Failed to parse timestamp '{}', using current time",
            timestamp_str
        );
        BsonDateTime::from_millis(Utc::now().timestamp_millis())
    })
}

// ========== FROM CREATE IMPLEMENTATIONS (UPDATED) ==========

// Helper function to create a Vote from CreateVote
impl Vote {
    pub fn from_create_vote(create_vote: CreateVote) -> Self {
        Vote {
            id: None,
            voter_id: create_vote.voter_id,
            username: create_vote.username,
            fixture_id: create_vote.fixture_id,
            home_team: create_vote.home_team,
            away_team: create_vote.away_team,
            draw: create_vote.draw,
            selection: create_vote.selection,
            vote_timestamp: BsonDateTime::from_millis(Utc::now().timestamp_millis()),
            created_at: Some(BsonDateTime::from_millis(Utc::now().timestamp_millis())),
        }
    }
}

// Helper function to create a Like from CreateLike
impl Like {
    pub fn from_create_like(create_like: CreateLike) -> Self {
        // Use current time instead of trying to parse a timestamp
        // Since the Flutter app doesn't send a timestamp for likes
        let current_time = BsonDateTime::from_millis(Utc::now().timestamp_millis());

        println!("📝 Creating like for fixture: {}, user: {}, action: {}",
                 create_like.fixture_id, create_like.voter_id, create_like.action);

        Like {
            id: None,
            voter_id: create_like.voter_id,
            username: create_like.username,
            fixture_id: create_like.fixture_id,
            action: create_like.action,
            like_timestamp: current_time,
            created_at: Some(current_time),
        }
    }
}

// Helper function to create a Comment from CreateComment - UPDATED WITH SELECTION
impl Comment {
    pub fn from_create_comment(create_comment: CreateComment) -> Result<Self, String> {
        // Validate selection
        let valid_selections = vec!["home_team", "draw", "away_team"];
        if !valid_selections.contains(&create_comment.selection.as_str()) {
            return Err(format!(
                "Invalid selection: {}. Must be one of: home_team, draw, away_team",
                create_comment.selection
            ));
        }

        // Try to parse the timestamp, but if it fails, use current time and log warning
        let comment_timestamp = match parse_iso_timestamp(&create_comment.timestamp) {
            Ok(ts) => ts,
            Err(e) => {
                println!("⚠️ Timestamp parsing failed: {}, using current time", e);
                BsonDateTime::from_millis(Utc::now().timestamp_millis())
            }
        };

        Ok(Comment {
            id: None,
            voter_id: create_comment.voter_id,
            username: create_comment.username,
            fixture_id: create_comment.fixture_id,
            selection: create_comment.selection, // NEW: Store the user's vote
            comment: create_comment.comment,
            timestamp: create_comment.timestamp,
            comment_timestamp,
            created_at: Some(BsonDateTime::from_millis(Utc::now().timestamp_millis())),
            likes: Some(0),
            replies: Some(Vec::new()),
        })
    }
}

// ========== SERIALIZATION HELPERS ==========

// Helper for BsonDateTime serialization
pub fn bson_datetime_to_iso_string(dt: &BsonDateTime) -> String {
    let millis = dt.timestamp_millis();
    let dt_chrono = Utc
        .timestamp_millis_opt(millis)
        .single()
        .unwrap_or_else(|| Utc::now());
    dt_chrono.to_rfc3339()
}

// Helper for optional BsonDateTime serialization
pub fn option_bson_datetime_to_iso_string(dt: &Option<BsonDateTime>) -> Option<String> {
    dt.as_ref().map(bson_datetime_to_iso_string)
}

// ========== RESPONSE WRAPPERS ==========

// For responses that need to be compatible with existing Game API
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// For compatibility with your existing game handlers
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u64,
    pub limit: i64,
}

// Error response structure
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub message: String,
    pub timestamp: String,
}

impl ErrorResponse {
    pub fn new(error: &str, message: &str) -> Self {
        ErrorResponse {
            success: false,
            error: error.to_string(),
            message: message.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

// ========== TOTAL COUNTS MODELS ==========

// Total counts for all fixtures
#[derive(Debug, Serialize, Deserialize)]
pub struct TotalCounts {
    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(rename = "totalLikes")]
    pub total_likes: i64,

    #[serde(rename = "totalComments")]
    pub total_comments: i64,

    #[serde(rename = "totalEngagement")]
    pub total_engagement: i64,

    #[serde(rename = "totalUsers")]
    pub total_users: i64,

    #[serde(rename = "timestamp")]
    pub timestamp: String,
}

// Response for total counts
#[derive(Debug, Serialize)]
pub struct TotalCountsResponse {
    pub success: bool,
    pub message: String,
    pub data: TotalCounts,
}

// Counts for a specific fixture
#[derive(Debug, Serialize, Deserialize)]
pub struct FixtureCounts {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,

    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(rename = "homeVotes")]
    pub home_votes: i64,

    #[serde(rename = "drawVotes")]
    pub draw_votes: i64,

    #[serde(rename = "awayVotes")]
    pub away_votes: i64,

    #[serde(rename = "totalLikes")]
    pub total_likes: i64,

    #[serde(rename = "totalComments")]
    pub total_comments: i64,

    #[serde(rename = "totalEngagement")]
    pub total_engagement: i64,

    #[serde(rename = "userHasVoted")]
    pub user_has_voted: bool,

    #[serde(rename = "userHasLiked")]
    pub user_has_liked: bool,

    #[serde(rename = "userSelection")]
    pub user_selection: Option<String>,
}

// Response for fixture counts
#[derive(Debug, Serialize)]
pub struct FixtureCountsResponse {
    pub success: bool,
    pub message: String,
    pub data: FixtureCounts,
}

// Batch request for multiple fixtures
#[derive(Debug, Deserialize)]
pub struct BatchFixtureCountsRequest {
    #[serde(rename = "fixtureIds")]
    pub fixture_ids: Vec<String>,

    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

// Single fixture count item for batch response
#[derive(Debug, Serialize, Deserialize)]
pub struct FixtureCountItem {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,

    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(rename = "totalLikes")]
    pub total_likes: i64,

    #[serde(rename = "totalComments")]
    pub total_comments: i64,

    #[serde(rename = "totalEngagement")]
    pub total_engagement: i64,

    #[serde(rename = "userHasVoted", skip_serializing_if = "Option::is_none")]
    pub user_has_voted: Option<bool>,

    #[serde(rename = "userHasLiked", skip_serializing_if = "Option::is_none")]
    pub user_has_liked: Option<bool>,

    #[serde(rename = "userSelection", skip_serializing_if = "Option::is_none")]
    pub user_selection: Option<String>,
}

// Batch response for multiple fixtures
#[derive(Debug, Serialize)]
pub struct BatchFixtureCountsResponse {
    pub success: bool,
    pub message: String,
    pub data: Vec<FixtureCountItem>,
    pub count: usize,
}

// Vote breakdown for detailed stats
#[derive(Debug, Serialize, Deserialize)]
pub struct VoteBreakdown {
    #[serde(rename = "homeVotes")]
    pub home_votes: i64,

    #[serde(rename = "drawVotes")]
    pub draw_votes: i64,

    #[serde(rename = "awayVotes")]
    pub away_votes: i64,

    #[serde(rename = "homePercentage")]
    pub home_percentage: f64,

    #[serde(rename = "drawPercentage")]
    pub draw_percentage: f64,

    #[serde(rename = "awayPercentage")]
    pub away_percentage: f64,
}

// Detailed fixture stats including vote breakdown
#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedFixtureStats {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,

    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(rename = "voteBreakdown")]
    pub vote_breakdown: VoteBreakdown,

    #[serde(rename = "totalLikes")]
    pub total_likes: i64,

    #[serde(rename = "totalComments")]
    pub total_comments: i64,

    #[serde(rename = "totalEngagement")]
    pub total_engagement: i64,

    #[serde(rename = "userHasVoted")]
    pub user_has_voted: bool,

    #[serde(rename = "userHasLiked")]
    pub user_has_liked: bool,

    #[serde(rename = "userSelection")]
    pub user_selection: Option<String>,
}

// Response for detailed stats
#[derive(Debug, Serialize)]
pub struct DetailedFixtureStatsResponse {
    pub success: bool,
    pub message: String,
    pub data: DetailedFixtureStats,
}

// ========== ADDITIONAL MODELS ==========

// User activity summary
#[derive(Debug, Serialize, Deserialize)]
pub struct UserActivitySummary {
    #[serde(rename = "voterId")]
    pub voter_id: String,

    #[serde(rename = "username")]
    pub username: String,

    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(rename = "totalLikes")]
    pub total_likes: i64,

    #[serde(rename = "totalComments")]
    pub total_comments: i64,

    #[serde(rename = "firstActivity", skip_serializing_if = "Option::is_none")]
    pub first_activity: Option<BsonDateTime>,

    #[serde(rename = "lastActivity", skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<BsonDateTime>,

    #[serde(rename = "favoriteTeams")]
    pub favorite_teams: Vec<String>,

    #[serde(rename = "mostVotedSelection", skip_serializing_if = "Option::is_none")]
    pub most_voted_selection: Option<String>,
}

// Fixture summary with engagement metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct FixtureEngagement {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,
    pub league: String,
    pub date: String,

    #[serde(rename = "totalEngagement")]
    pub total_engagement: i64,

    #[serde(rename = "voteEngagement")]
    pub vote_engagement: i64,

    #[serde(rename = "likeEngagement")]
    pub like_engagement: i64,

    #[serde(rename = "commentEngagement")]
    pub comment_engagement: i64,

    #[serde(rename = "engagementScore")]
    pub engagement_score: f64,

    #[serde(rename = "trendingRank")]
    pub trending_rank: i32,
}

// Real-time vote update
#[derive(Debug, Serialize, Deserialize)]
pub struct VoteUpdate {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeVotes")]
    pub home_votes: i64,

    #[serde(rename = "drawVotes")]
    pub draw_votes: i64,

    #[serde(rename = "awayVotes")]
    pub away_votes: i64,

    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(rename = "updateTimestamp")]
    pub update_timestamp: BsonDateTime,
}

// Like with user info
#[derive(Debug, Serialize, Deserialize)]
pub struct LikeWithUser {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    #[serde(rename = "voterId")]
    pub voter_id: String,

    #[serde(rename = "username")]
    pub username: String,

    #[serde(rename = "fixtureId")]
    pub fixture_id: String,
    pub action: String,

    #[serde(rename = "likeTimestamp")]
    pub like_timestamp: BsonDateTime,

    #[serde(rename = "userDisplayName", skip_serializing_if = "Option::is_none")]
    pub user_display_name: Option<String>,

    #[serde(rename = "userAvatar", skip_serializing_if = "Option::is_none")]
    pub user_avatar: Option<String>,
}

// Batch update request for multiple fixtures
#[derive(Debug, Deserialize)]
pub struct BatchStatsRequest {
    #[serde(rename = "fixtureIds")]
    pub fixture_ids: Vec<String>,

    #[serde(rename = "includeVotes")]
    pub include_votes: Option<bool>,

    #[serde(rename = "includeLikes")]
    pub include_likes: Option<bool>,

    #[serde(rename = "includeComments")]
    pub include_comments: Option<bool>,
}

// Batch stats response
#[derive(Debug, Serialize)]
pub struct BatchStatsResponse {
    pub success: bool,
    pub data: Vec<FixtureStats>,

    #[serde(rename = "totalFixtures")]
    pub total_fixtures: usize,
    pub timestamp: String,
}

// Popular fixture (for trending)
#[derive(Debug, Serialize, Deserialize)]
pub struct PopularFixture {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,
    pub league: String,
    pub date: String,

    #[serde(rename = "totalEngagement")]
    pub total_engagement: i64,

    #[serde(rename = "voteCount")]
    pub vote_count: i64,

    #[serde(rename = "likeCount")]
    pub like_count: i64,

    #[serde(rename = "commentCount")]
    pub comment_count: i64,
    pub rank: i32,
}

// User vote history
#[derive(Debug, Serialize, Deserialize)]
pub struct UserVoteHistory {
    #[serde(rename = "voterId")]
    pub voter_id: String,

    #[serde(rename = "username")]
    pub username: String,

    pub votes: Vec<UserVoteEntry>,

    #[serde(rename = "totalVotes")]
    pub total_votes: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserVoteEntry {
    #[serde(rename = "fixtureId")]
    pub fixture_id: String,

    #[serde(rename = "homeTeam")]
    pub home_team: String,

    #[serde(rename = "awayTeam")]
    pub away_team: String,
    pub selection: String,

    #[serde(rename = "voteTimestamp")]
    pub vote_timestamp: BsonDateTime,

    #[serde(rename = "wasCorrect", skip_serializing_if = "Option::is_none")]
    pub was_correct: Option<bool>,
}

// ========== DEFAULT IMPLEMENTATIONS ==========

impl Default for VoteResponse {
    fn default() -> Self {
        Self {
            success: false,
            message: String::new(),
            vote_id: None,
            data: None,
        }
    }
}

impl Default for LikeResponse {
    fn default() -> Self {
        Self {
            success: false,
            message: String::new(),
            like_id: None,
            total_likes: 0,
        }
    }
}

impl Default for CommentResponse {
    fn default() -> Self {
        Self {
            success: false,
            message: String::new(),
            comment_id: None,
            comment: None,
        }
    }
}
