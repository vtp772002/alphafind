use crate::models::CandidateAlpha;

/// The 6 Canonical Economic Factor Pillars of WorldQuant BRAIN
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorPillar {
    AnalystConsensus,
    OptionsVrp,
    MicrostructureVwap,
    FinancialHealthQuality,
    IdiosyncraticRisk,
    ShortInterest,
}

impl FactorPillar {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AnalystConsensus => "ANALYST_CONSENSUS",
            Self::OptionsVrp => "OPTIONS_VRP",
            Self::MicrostructureVwap => "MICROSTRUCTURE_VWAP",
            Self::FinancialHealthQuality => "FINANCIAL_HEALTH_QUALITY",
            Self::IdiosyncraticRisk => "IDIOSYNCRATIC_RISK",
            Self::ShortInterest => "SHORT_INTEREST",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let upper = s.to_uppercase();
        if upper.contains("ANALYST") {
            Some(Self::AnalystConsensus)
        } else if upper.contains("OPT") || upper.contains("VRP") {
            Some(Self::OptionsVrp)
        } else if upper.contains("MICRO") || upper.contains("VWAP") {
            Some(Self::MicrostructureVwap)
        } else if upper.contains("QUAL") || upper.contains("FUND") || upper.contains("HEALTH") {
            Some(Self::FinancialHealthQuality)
        } else if upper.contains("RISK") || upper.contains("IDIO") {
            Some(Self::IdiosyncraticRisk)
        } else if upper.contains("SHORT") || upper.contains("SQUEEZE") {
            Some(Self::ShortInterest)
        } else {
            None
        }
    }
}

/// Generates a curated batch of 9 orthogonal candidate alphas for parallel distributed screening
pub fn get_curated_candidates(pillar: Option<FactorPillar>, universe: Option<&str>) -> Vec<CandidateAlpha> {
    let u = universe.unwrap_or("TOP1000");

    match pillar {
        Some(FactorPillar::AnalystConsensus) => vec![
            CandidateAlpha {
                name: "US_D1_TOP500_Analyst_EPS_Delta5_Dec32".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(actual_eps_value_quarterly, 20) / close) - 0.5, 3.2) - 0.6 * rank(ts_delta(close, 5)), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 32,
                neutralization: "SECTOR".to_string(),
                truncation: 0.07,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_Dual_Consensus_Yield_Dec40".to_string(),
                expression: "group_neutralize(signed_power(0.6 * rank(ts_backfill(actual_eps_value_quarterly, 60) / close) + 0.4 * rank(ts_backfill(actual_adj_net_income_quarterly, 60) / cap) - 0.5, 4.0), densify(market))".to_string(),
                universe: "TOP500".to_string(),
                decay: 40,
                neutralization: "MARKET".to_string(),
                truncation: 0.065,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_PureAnalyst_Apex_Dec50".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_decay_linear(ts_backfill(actual_eps_value_quarterly / close, 126), 20)) - 0.5, 4.4), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 50,
                neutralization: "SECTOR".to_string(),
                truncation: 0.065,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
        ],
        Some(FactorPillar::IdiosyncraticRisk) => vec![
            CandidateAlpha {
                name: "US_D1_TOP1000_IdioCurv_30_60_90_Dec14".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(unsystematic_risk_last_30_days, 10) - 2.0 * ts_backfill(unsystematic_risk_last_60_days, 10) + ts_backfill(unsystematic_risk_last_90_days, 10)) - 0.5, 2.5), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 14,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "IDIOSYNCRATIC_RISK".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_IdioVol_60d_Delta3_Dec16".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(unsystematic_risk_last_60_days, 15)) - 0.5, 2.8) - 0.5 * rank(ts_delta(close, 3)), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 16,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "IDIOSYNCRATIC_RISK".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_IdioRisk_Spread_Dec12".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(unsystematic_risk_last_30_days / unsystematic_risk_last_90_days, 10)) - 0.5, 3.0), densify(subindustry))".to_string(),
                universe: "TOP500".to_string(),
                decay: 12,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "IDIOSYNCRATIC_RISK".to_string(),
            },
        ],
        Some(FactorPillar::OptionsVrp) => vec![
            CandidateAlpha {
                name: "US_D1_TOP3000_VRP120d_Hedge_Dec26".to_string(),
                expression: "vrp = rank(ts_backfill(implied_volatility_call_120 / historical_volatility_120, 20)) - rank(ts_backfill(implied_volatility_put_120 / historical_volatility_120, 20)); signal = signed_power(vrp, 1.3) - 0.04 * signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 26,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "OPTIONS_VRP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_VRP60d_Delta3_Dec18".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(implied_volatility_call_60 / historical_volatility_60, 15)) - rank(ts_backfill(implied_volatility_put_60 / historical_volatility_60, 15)) - 0.5, 3.8), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 18,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "OPTIONS_VRP".to_string(),
            },
        ],
        _ => vec![
            // Default 9-worker distributed batch across 3 accounts
            CandidateAlpha {
                name: "US_D1_TOP500_Analyst_EPS_Delta5_Dec32".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(actual_eps_value_quarterly, 20) / close) - 0.5, 3.2) - 0.6 * rank(ts_delta(close, 5)), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 32,
                neutralization: "SECTOR".to_string(),
                truncation: 0.07,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_IdioCurv_30_60_90_Dec14".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_backfill(unsystematic_risk_last_30_days, 10) - 2.0 * ts_backfill(unsystematic_risk_last_60_days, 10) + ts_backfill(unsystematic_risk_last_90_days, 10)) - 0.5, 2.5), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 14,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.065,
                pillar: "IDIOSYNCRATIC_RISK".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_VRP120d_Hedge_Dec26".to_string(),
                expression: "vrp = rank(ts_backfill(implied_volatility_call_120 / historical_volatility_120, 20)) - rank(ts_backfill(implied_volatility_put_120 / historical_volatility_120, 20)); signal = signed_power(vrp, 1.3) - 0.04 * signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 26,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "OPTIONS_VRP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_Micro_VWAP_Burst_Dec40".to_string(),
                expression: "group_neutralize(signed_power(rank(vwap / close - 1) - 0.5, 3.8) * (1.0 + 0.5 * rank(volume / ts_mean(volume, 40))), densify(market))".to_string(),
                universe: "TOP500".to_string(),
                decay: 40,
                neutralization: "MARKET".to_string(),
                truncation: 0.065,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_Sloan_Accruals_Dec16".to_string(),
                expression: "group_neutralize(signed_power(rank((cashflow_op - income) / assets) - 0.5, 3.5), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 16,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "FINANCIAL_HEALTH_QUALITY".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP500_PureAnalyst_Apex_Dec50".to_string(),
                expression: "group_neutralize(signed_power(rank(ts_decay_linear(ts_backfill(actual_eps_value_quarterly / close, 126), 20)) - 0.5, 4.4), densify(sector))".to_string(),
                universe: "TOP500".to_string(),
                decay: 50,
                neutralization: "SECTOR".to_string(),
                truncation: 0.065,
                pillar: "ANALYST_CONSENSUS".to_string(),
            },
        ],
    }
}
