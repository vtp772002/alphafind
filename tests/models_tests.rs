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
