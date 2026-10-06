use alphafind::models::{AlphaDetails, AlphaSettings, InSampleStats, SimulationResponse};

#[test]
fn test_alpha_settings_defaults() {
    let settings = AlphaSettings::default();
    assert_eq!(settings.instrument_type, "EQUITY");
    assert_eq!(settings.region, "USA");
    assert_eq!(settings.universe, "TOP1000");
    assert_eq!(settings.delay, 1);
    assert_eq!(settings.decay, 5);
    assert_eq!(settings.neutralization, "SUBINDUSTRY");
    assert_eq!(settings.truncation, 0.05);
    assert_eq!(settings.pasteurization, "ON");
    assert_eq!(settings.unit_handling, "VERIFY");
    assert_eq!(settings.nan_handling, "OFF");
    assert_eq!(settings.language, "FASTEXPR");
    assert!(!settings.visualization);
}

#[test]
fn test_alpha_settings_serialization() {
    let settings = AlphaSettings::default();
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"instrumentType\""));
    assert!(json.contains("\"unitHandling\""));
    // Verify camelCase renaming
    assert!(!json.contains("instrument_type"));
}

#[test]
fn test_alpha_settings_deserialization() {
    let json = r#"{
        "instrumentType": "EQUITY",
        "region": "USA",
        "universe": "TOP500",
        "delay": 1,
        "decay": 10,
        "neutralization": "MARKET",
        "truncation": 0.065,
        "pasteurization": "ON",
        "unitHandling": "VERIFY",
        "nanHandling": "OFF",
        "language": "FASTEXPR",
        "visualization": false
    }"#;
    let settings: AlphaSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.universe, "TOP500");
    assert_eq!(settings.decay, 10);
    assert_eq!(settings.neutralization, "MARKET");
    assert_eq!(settings.truncation, 0.065);
}

#[test]
fn test_alpha_settings_partial_deserialization() {
    // Test that missing fields get defaults
    let json = r#"{"universe": "TOP3000", "decay": 20}"#;
    let settings: AlphaSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.universe, "TOP3000");
    assert_eq!(settings.decay, 20);
    assert_eq!(settings.region, "USA"); // default
    assert_eq!(settings.delay, 1); // default
}

#[test]
fn test_simulation_response_deserialization() {
    let json = r#"{
        "id": "sim_123",
        "status": "COMPLETE",
        "alpha": "alphaABC"
    }"#;
    let resp: SimulationResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some("COMPLETE".to_string()));
    assert_eq!(resp.alpha, Some("alphaABC".to_string()));
}

#[test]
fn test_simulation_response_error() {
    let json = r#"{
        "status": "ERROR",
        "message": "Invalid expression"
    }"#;
    let resp: SimulationResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some("ERROR".to_string()));
    assert_eq!(resp.message, Some("Invalid expression".to_string()));
    assert!(resp.alpha.is_none());
}

#[test]
fn test_in_sample_stats_defaults() {
    let stats = InSampleStats::default();
    assert!(stats.sharpe.is_none());
    assert!(stats.fitness.is_none());
    assert!(stats.checks.is_empty());
    assert!(stats.self_correlated.is_none());
}

#[test]
fn test_in_sample_stats_deserialization() {
    let json = r#"{
        "sharpe": 1.45,
        "fitness": 1.62,
        "returns": 0.18,
        "turnover": 0.22,
        "margin": 0.0015,
        "drawdown": 0.08,
        "checks": [
            {"name": "LOW_SHARPE", "result": "PASS"},
            {"name": "LOW_FITNESS", "result": "PASS"}
        ]
    }"#;
    let stats: InSampleStats = serde_json::from_str(json).unwrap();
    assert_eq!(stats.sharpe, Some(1.45));
    assert_eq!(stats.fitness, Some(1.62));
    assert_eq!(stats.checks.len(), 2);
    assert_eq!(stats.checks[0].name, "LOW_SHARPE");
    assert_eq!(stats.checks[0].result, "PASS");
}

#[test]
fn test_alpha_details_deserialization() {
    let json = r#"{
        "id": "abc123",
        "name": "Test Alpha",
        "category": "FUNDAMENTAL",
        "is": {
            "sharpe": 1.50,
            "fitness": 1.75,
            "returns": 0.20,
            "turnover": 0.25,
            "checks": []
        }
    }"#;
    let details: AlphaDetails = serde_json::from_str(json).unwrap();
    assert_eq!(details.id, "abc123");
    assert_eq!(details.name, Some("Test Alpha".to_string()));
    assert!(details.is.is_some());
    let is = details.is.unwrap();
    assert_eq!(is.sharpe, Some(1.50));
}

#[test]
fn test_user_profile_deserialization() {
    let json = r#"{
        "id": "USR12345",
        "email": "user@example.com",
        "firstName": "Alex",
        "lastName": "Quant",
        "fullName": "Alex Quant",
        "level": "GOLD"
    }"#;
    let profile: alphafind::models::UserProfile = serde_json::from_str(json).unwrap();
    assert_eq!(profile.id, "USR12345");
    assert_eq!(profile.email, "user@example.com");
    assert_eq!(profile.full_name, Some("Alex Quant".to_string()));
    assert_eq!(profile.level, Some("GOLD".to_string()));
}

#[test]
fn test_competition_entry_deserialization() {
    let json = r#"{
        "id": "challenge",
        "name": "Challenge - Global",
        "status": "ACCEPTED",
        "scoring": "PERFORMANCE",
        "leaderboard": {
            "rank": 2,
            "score": 0.74,
            "daysOfSubmission": 22,
            "isScore": 8887.0,
            "uniquenessScore": 0.50,
            "university": "Institute of Technology"
        }
    }"#;
    let comp: alphafind::models::CompetitionEntry = serde_json::from_str(json).unwrap();
    assert_eq!(comp.id, "challenge");
    assert_eq!(comp.name, "Challenge - Global");
    assert!(comp.leaderboard.is_some());
    let lb = comp.leaderboard.unwrap();
    assert_eq!(lb.rank, Some(2));
    assert_eq!(lb.score, Some(0.74));
    assert_eq!(lb.days_of_submission, Some(22));
    assert_eq!(lb.is_score, Some(8887.0));
    assert_eq!(lb.uniqueness_score, Some(0.50));
}

#[test]
fn test_leaderboard_response_deserialization() {
    let json = r#"{
        "count": 52,
        "results": [
            {
                "rank": 1,
                "user": "USR99001",
                "score": 0.8,
                "daysOfSubmission": 10,
                "isScore": 21679.0,
                "uniquenessScore": -0.03,
                "university": "Institute of Technology"
            },
            {
                "rank": 2,
                "user": {
                    "id": "USR12345",
                    "name": null,
                    "image": null
                },
                "score": 0.74,
                "daysOfSubmission": 22,
                "isScore": 8887.0,
                "uniquenessScore": 0.5,
                "university": "Institute of Technology"
            }
        ]
    }"#;
    let resp: alphafind::models::LeaderboardResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.count, 52);
    assert_eq!(resp.results.len(), 2);
    assert_eq!(resp.results[0].rank, 1);
    assert_eq!(resp.results[0].user.id(), "USR99001");
    assert_eq!(resp.results[0].score, 0.8);
    assert_eq!(resp.results[1].rank, 2);
    assert_eq!(resp.results[1].user.id(), "USR12345");
    assert_eq!(resp.results[1].score, 0.74);
}


