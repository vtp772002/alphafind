use serde::{Deserialize, Serialize};

/// Standard simulation settings for WorldQuant BRAIN platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaSettings {
    #[serde(default = "default_instrument_type")]
    pub instrument_type: String,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_universe")]
    pub universe: String,
    #[serde(default = "default_delay")]
    pub delay: i32,
    #[serde(default = "default_decay")]
    pub decay: i32,
    #[serde(default = "default_neutralization")]
    pub neutralization: String,
    #[serde(default = "default_truncation")]
    pub truncation: f64,
    #[serde(default = "default_pasteurization")]
    pub pasteurization: String,
    #[serde(default = "default_unit_handling")]
    pub unit_handling: String,
    #[serde(default = "default_nan_handling")]
    pub nan_handling: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub visualization: bool,
}

fn default_instrument_type() -> String {
    "EQUITY".to_string()
}
fn default_region() -> String {
    "USA".to_string()
}
fn default_universe() -> String {
    "TOP1000".to_string()
}
fn default_delay() -> i32 {
    1
}
fn default_decay() -> i32 {
    5
}
fn default_neutralization() -> String {
    "SUBINDUSTRY".to_string()
}
fn default_truncation() -> f64 {
    0.05
}
fn default_pasteurization() -> String {
    "ON".to_string()
}
fn default_unit_handling() -> String {
    "VERIFY".to_string()
}
fn default_nan_handling() -> String {
    "OFF".to_string()
}
fn default_language() -> String {
    "FASTEXPR".to_string()
}

impl Default for AlphaSettings {
    fn default() -> Self {
        Self {
            instrument_type: default_instrument_type(),
            region: default_region(),
            universe: default_universe(),
            delay: default_delay(),
            decay: default_decay(),
            neutralization: default_neutralization(),
            truncation: default_truncation(),
            pasteurization: default_pasteurization(),
            unit_handling: default_unit_handling(),
            nan_handling: default_nan_handling(),
            language: default_language(),
            visualization: false,
        }
    }
}

/// Simulation submission payload
#[derive(Debug, Serialize)]
pub struct SimulationPayload<'a> {
    #[serde(rename = "type")]
    pub sim_type: &'a str,
    pub settings: &'a AlphaSettings,
    pub regular: &'a str,
}

/// Simulation polling response
#[derive(Debug, Clone, Deserialize)]
pub struct SimulationResponse {
    pub id: Option<String>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub alpha: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "retry-after")]
    pub retry_after: Option<f64>,
}

/// Individual check item from BRAIN IS verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    pub name: String,
    pub result: String,
    pub value: Option<serde_json::Value>,
    pub limit: Option<serde_json::Value>,
}

/// Self-correlation container from /check endpoint
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelfCorrelated {
    pub max: Option<f64>,
    pub records: Option<Vec<Vec<serde_json::Value>>>,
}

/// In-Sample statistics returned by /alphas/{id} and /check
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InSampleStats {
    pub sharpe: Option<f64>,
    pub fitness: Option<f64>,
    pub returns: Option<f64>,
    pub turnover: Option<f64>,
    pub pnl: Option<f64>,
    pub margin: Option<f64>,
    pub book_size: Option<f64>,
    pub drawdown: Option<f64>,
    #[serde(default)]
    pub checks: Vec<CheckItem>,
    pub self_correlated: Option<SelfCorrelated>,
}

/// Complete Alpha details from GET /alphas/{id}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaDetails {
    pub id: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub color: Option<String>,
    pub tags: Option<Vec<String>>,
    pub settings: Option<AlphaSettings>,
    #[serde(default)]
    pub regular: Option<serde_json::Value>,
    pub is: Option<InSampleStats>,
}

impl AlphaDetails {
    pub fn get_code(&self) -> Option<&str> {
        self.regular.as_ref().and_then(|r| {
            if let Some(s) = r.as_str() {
                Some(s)
            } else {
                r.get("code").and_then(|c| c.as_str())
            }
        })
    }
}

/// Daily PnL records returned by /alphas/{id}/recordsets/daily-pnl
#[derive(Debug, Clone, Deserialize)]
pub struct DailyPnlResponse {
    #[serde(default)]
    pub records: Vec<Vec<serde_json::Value>>,
}

/// Active Out-of-Sample Alpha stored in portfolio_os.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAlpha {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub date_submitted: Option<String>,
    #[serde(default)]
    pub universe: Option<String>,
    #[serde(default)]
    pub decay: Option<i32>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub sharpe: Option<f64>,
    #[serde(default)]
    pub fitness: Option<f64>,
    #[serde(default)]
    pub returns: Option<f64>,
    #[serde(default)]
    pub turnover: Option<f64>,
    #[serde(default)]
    pub direct_url: Option<String>,
    #[serde(default, alias = "expression")]
    pub code: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
}

/// Submittable Alpha Record in submittable_alphas.csv
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmittableAlphaRecord {
    #[serde(rename = "Alpha ID")]
    pub alpha_id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Universe")]
    pub universe: String,
    #[serde(rename = "Sharpe")]
    pub sharpe: f64,
    #[serde(rename = "Fitness")]
    pub fitness: f64,
    #[serde(rename = "Turnover (%)")]
    pub turnover: f64,
    #[serde(rename = "Annual Return (%)")]
    pub annual_return: f64,
    #[serde(rename = "Submitted")]
    pub submitted: bool,
    #[serde(rename = "Expression")]
    pub expression: String,
    #[serde(rename = "Settings")]
    pub settings: String,
}

/// Candidate Alpha for hypothesis generation and screening
#[derive(Debug, Clone)]
pub struct CandidateAlpha {
    pub name: String,
    pub expression: String,
    pub universe: String,
    pub decay: i32,
    pub neutralization: String,
    pub truncation: f64,
    pub pillar: String,
}

impl CandidateAlpha {
    pub fn to_settings(&self) -> AlphaSettings {
        AlphaSettings {
            universe: self.universe.clone(),
            decay: self.decay,
            neutralization: self.neutralization.clone(),
            truncation: self.truncation,
            ..Default::default()
        }
    }
}

/// User profile returned by /users/self
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub full_name: Option<String>,
    pub level: Option<String>,
}

/// User competition leaderboard metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserLeaderboard {
    pub rank: Option<i64>,
    pub score: Option<f64>,
    pub days_of_submission: Option<i64>,
    pub is_score: Option<f64>,
    pub uniqueness_score: Option<f64>,
    pub university: Option<String>,
}

/// Competition entry returned by /competitions or /users/self/competitions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionEntry {
    pub id: String,
    pub name: String,
    pub status: Option<String>,
    pub scoring: Option<String>,
    pub leaderboard: Option<UserLeaderboard>,
}

/// User representation in competition leaderboard board
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LeaderboardUser {
    Id(String),
    Detailed {
        id: String,
        name: Option<serde_json::Value>,
        image: Option<serde_json::Value>,
    },
}

impl LeaderboardUser {
    pub fn id(&self) -> &str {
        match self {
            LeaderboardUser::Id(id) => id,
            LeaderboardUser::Detailed { id, .. } => id,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            LeaderboardUser::Id(id) => id.clone(),
            LeaderboardUser::Detailed { id, name, .. } => {
                if let Some(n) = name {
                    if let Some(fn_str) = n.get("fullName").and_then(|v| v.as_str()) {
                        return format!("{} ({})", fn_str, id);
                    }
                }
                id.clone()
            }
        }
    }
}

/// Individual standing in /competitions/{id}/boards/leader
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user: LeaderboardUser,
    pub score: f64,
    pub days_of_submission: i64,
    pub is_score: f64,
    pub uniqueness_score: f64,
    pub university: Option<String>,
}

/// Response returned by /competitions/{id}/boards/leader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub count: usize,
    pub results: Vec<LeaderboardEntry>,
}

/// Category / subcategory info in DatasetEntry
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatasetCategoryInfo {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// A dataset entry returned by /data-sets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetEntry {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<DatasetCategoryInfo>,
    #[serde(default)]
    pub subcategory: Option<DatasetCategoryInfo>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub delay: Option<i32>,
    #[serde(default)]
    pub universe: Option<String>,
    #[serde(default)]
    pub date_coverage: Option<f64>,
    #[serde(default)]
    pub coverage: Option<f64>,
    #[serde(default)]
    pub value_score: Option<f64>,
    #[serde(default)]
    pub user_count: Option<i64>,
    #[serde(default)]
    pub alpha_count: Option<i64>,
    #[serde(default)]
    pub field_count: Option<i64>,
}

/// Result of multi-dimensional hyperparameter autotuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunedAlphaResult {
    pub details: AlphaDetails,
    pub expression: String,
    pub settings: AlphaSettings,
    pub original_fitness: f64,
    pub tuned_fitness: f64,
    pub tuning_strategy: String,
}

/// Response returned by /data-sets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetListResponse {
    pub count: usize,
    #[serde(default)]
    pub results: Vec<DatasetEntry>,
}

/// Historical snapshot of dataset census for tracking crowd migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CensusSnapshot {
    pub timestamp: String,
    pub region: String,
    pub total_datasets: usize,
    pub datasets: Vec<DatasetEntry>,
}

/// Portfolio Impact report for a candidate alpha
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioImpactResult {
    pub candidate_id: String,
    pub baseline_alphas: usize,
    pub baseline_sharpe: f64,
    pub baseline_pnl: f64,
    pub baseline_vol: f64,
    pub baseline_avg_corr: f64,

    pub new_alphas: usize,
    pub new_sharpe: f64,
    pub new_pnl: f64,
    pub new_vol: f64,
    pub new_avg_corr: f64,

    pub delta_sharpe: f64,
    pub delta_pnl: f64,
    pub delta_vol: f64,
    pub delta_avg_corr: f64,

    pub max_pairwise_corr: f64,
    pub most_correlated_id: String,
    pub avg_pairwise_corr_vs_os: f64,
    pub safety_buffer_pct: f64,
}
