use serde::{Deserialize, Serialize};
use bson::DateTime as BsonDateTime;

// Main Game model - matches your MongoDB documents EXACTLY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    #[serde(rename = "_id")]
    pub id: String,  // Every document has an _id

    #[serde(rename = "match_id")]
    pub match_id: String,

    #[serde(rename = "home_team")]
    pub home_team: String,

    #[serde(rename = "away_team")]
    pub away_team: String,

    #[serde(rename = "league")]
    pub league: String,

    #[serde(rename = "home_win")]
    pub home_win: f64,

    #[serde(rename = "away_win")]
    pub away_win: f64,

    #[serde(rename = "draw")]
    pub draw: f64,

    #[serde(rename = "date")]
    pub date: String,

    #[serde(rename = "time")]
    pub time: String,

    #[serde(rename = "home_score", skip_serializing_if = "Option::is_none")]
    pub home_score: Option<i32>,

    #[serde(rename = "away_score", skip_serializing_if = "Option::is_none")]
    pub away_score: Option<i32>,

    #[serde(rename = "status")]
    pub status: String,

    #[serde(rename = "is_live")]
    pub is_live: bool,

    #[serde(rename = "available_for_voting")]
    pub available_for_voting: bool,

    #[serde(rename = "source")]
    pub source: String,

    #[serde(rename = "scraped_at")]
    pub scraped_at: BsonDateTime,

    #[serde(rename = "date_iso")]
    pub date_iso: String,
}

// For creating new games
#[derive(Debug, Deserialize)]
pub struct CreateGame {
    pub match_id: String,
    pub home_team: String,
    pub away_team: String,
    pub league: String,
    pub home_win: f64,
    pub away_win: f64,
    pub draw: f64,
    pub date: String,
    pub time: String,
    pub date_iso: String,
    pub source: String,
}

// For updating game scores
#[derive(Debug, Deserialize)]
pub struct UpdateGameScore {
    pub match_id: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub status: Option<String>,
    pub is_live: Option<bool>,
}

// For live game updates
#[derive(Debug, Deserialize)]
pub struct LiveUpdate {
    pub match_id: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
}

// For query parameters
#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub status: Option<String>,
    pub league: Option<String>,
    pub is_live: Option<bool>,
    pub limit: Option<i64>,
    pub skip: Option<u64>,
    pub source: Option<String>,
}

// Response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub message: Option<String>,
}

// For paginated responses
#[derive(Debug, Serialize)]
pub struct PaginatedGames {
    pub games: Vec<Game>,
    pub total: i64,
    pub page: u64,
    pub limit: i64,
}

// For live games response
#[derive(Debug, Serialize)]
pub struct LiveGamesResponse {
    pub live_games: Vec<Game>,
    pub count: usize,
    pub last_updated: BsonDateTime,
}

// For game statistics
#[derive(Debug, Serialize)]
pub struct GameStats {
    pub total_games: i64,
    pub upcoming_games: i64,
    pub live_games: i64,
    pub completed_games: i64,
    pub by_league: Vec<LeagueStats>,
}

#[derive(Debug, Serialize)]
pub struct LeagueStats {
    pub league: String,
    pub count: i64,
    pub upcoming: i64,
    pub live: i64,
    pub completed: i64,
}

// For bulk operations
#[derive(Debug, Deserialize)]
pub struct BulkGameUpdate {
    pub games: Vec<UpdateGameScore>,
}

// For game status updates
#[derive(Debug, Deserialize)]
pub struct GameStatusUpdate {
    pub match_id: String,
    pub status: String,
    pub is_live: bool,
}

// Helper implementations
impl Game {
    pub fn is_upcoming(&self) -> bool {
        self.status == "upcoming"
    }

    pub fn is_live_game(&self) -> bool {
        self.status == "live" || self.is_live
    }

    pub fn is_completed(&self) -> bool {
        self.status == "completed"
    }

    pub fn formatted_score(&self) -> String {
        match (self.home_score, self.away_score) {
            (Some(home), Some(away)) => format!("{} - {}", home, away),
            _ => "VS".to_string(),
        }
    }

    pub fn display_date(&self) -> String {
        if !self.date_iso.is_empty() {
            self.date_iso.clone()
        } else {
            format!("{} {}", self.date, self.time)
        }
    }
}
