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
    NewsEvent,
    CrossSanctuary,
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
            Self::NewsEvent => "NEWS_EVENT",
            Self::CrossSanctuary => "CROSS_SANCTUARY",
        }
    }

    #[allow(clippy::should_implement_trait)]
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
        } else if upper.contains("CROSS") || upper.contains("SANCTUARY") || upper.contains("HYBRID") {
            Some(Self::CrossSanctuary)
        } else if upper.contains("SHORT") || upper.contains("SQUEEZE") {
            Some(Self::ShortInterest)
        } else if upper.contains("NEWS") || upper.contains("SENTIMENT") {
            Some(Self::NewsEvent)
        } else {
            None
        }
    }
}

/// Generates a curated batch of 9 orthogonal candidate alphas for parallel distributed screening
pub fn get_curated_candidates(
    pillar: Option<FactorPillar>,
    universe: Option<&str>,
) -> Vec<CandidateAlpha> {
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
        Some(FactorPillar::MicrostructureVwap) => vec![
            CandidateAlpha {
                name: "US_D1_TOP3000_NightDay_Spread_SubNeut_Dec22".to_string(),
                expression: "night_ret = winsorize((open - ts_delay(close, 1)) / ts_delay(close, 1)); day_ret = winsorize((close - open) / open); spread = ts_mean(night_ret, 10) - ts_mean(day_ret, 10); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_ATR_NightDay_Spread_Dec20".to_string(),
                expression: "atr = ts_mean(high - low, 20); night_gap = winsorize((open - ts_delay(close, 1)) / atr); day_drift = winsorize((close - open) / atr); spread = ts_mean(night_gap, 10) - ts_mean(day_drift, 10); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 20,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_Micro_VWAP_Burst_Dec38".to_string(),
                expression: "vwap_dev = signed_power(rank(vwap / close - 1) - 0.5, 1.4); vol_burst = signed_power(rank(volume / ts_mean(volume, 60)) - 0.5, 1.3); reversal = signed_power(rank(ts_decay_linear(ts_delta(close, 5), 3)) - 0.5, 1.5); signal = vwap_dev * (1 + 0.70 * vol_burst) - 0.04 * reversal; group_neutralize(signed_power(signal, 3.8), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 38,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_NightDay_Spread_Dec22".to_string(),
                expression: "night_ret = winsorize((open - ts_delay(close, 1)) / ts_delay(close, 1)); day_ret = winsorize((close - open) / open); spread = ts_mean(night_ret, 10) - ts_mean(day_ret, 10); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_NightDay_VolWeight_Dec24".to_string(),
                expression: "night_ret = winsorize((open - ts_delay(close, 1)) / ts_delay(close, 1)); day_ret = winsorize((close - open) / open); spread = (ts_mean(night_ret, 10) - ts_mean(day_ret, 10)) * (1 + 0.5 * rank(volume / ts_mean(volume, 40))); signal = signed_power(rank(spread) - 0.5, 1.8); group_neutralize(signed_power(signal, 4.0), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 24,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_Micro_VWAP_Burst_Dec40".to_string(),
                expression: "vwap_dev = signed_power(rank(vwap / close - 1) - 0.5, 1.4); vol_burst = signed_power(rank(volume / ts_mean(volume, 60)) - 0.5, 1.3); reversal = signed_power(rank(ts_decay_linear(ts_delta(close, 5), 3)) - 0.5, 1.5); signal = vwap_dev * (1 + 0.70 * vol_burst) - 0.04 * reversal; group_neutralize(signed_power(signal, 3.8), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 40,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "MICROSTRUCTURE_VWAP".to_string(),
            },
        ],
        Some(FactorPillar::NewsEvent) => vec![
            CandidateAlpha {
                name: "US_D1_TOP3000_News_Event_FastRev_Dec20".to_string(),
                expression: "news = rank(ts_decay_linear(winsorize(ts_backfill(vec_avg(nws12_afterhsz_sl), 30)), 20)); rev = signed_power(rank(ts_decay_linear(ts_delta(close, 5), 3)) - 0.5, 1.5); signal = signed_power(news - 0.5, 1.5) - 0.04 * rev; group_neutralize(signed_power(signal, 4.0), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 20,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_EVENT".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_News_VolSurge_Dec24".to_string(),
                expression: "news = ts_backfill(vec_avg(nws12_afterhsz_sl), 20); f_news = if_else(is_nan(news), 0, rank(news) - 0.5); vol_ratio = rank(volume / ts_mean(volume, 30)) - 0.5; signal = signed_power(f_news, 1.5) * (1 + 0.65 * signed_power(vol_ratio, 1.2)); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 24,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_EVENT".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP1000_News_Afterhours_Dec22".to_string(),
                expression: "news = rank(ts_decay_linear(winsorize(ts_backfill(vec_avg(nws12_afterhsz_sl), 25)), 15)); rev = signed_power(rank(ts_delta(close, 4)) - 0.5, 1.5); signal = signed_power(news - 0.5, 1.6) - 0.03 * rev; group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "NEWS_EVENT".to_string(),
            },
        ],
        Some(FactorPillar::ShortInterest) => vec![
            CandidateAlpha {
                name: "US_D1_TOP1000_Short_Dual_SubNeut_Dec28".to_string(),
                expression: "short_cov = ts_backfill(vec_avg(shares_sold_short_count_2), 45); short_all = ts_backfill(vec_avg(shorted_shares_count_all), 45); f_cov = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); f_all = if_else(is_nan(short_all), 0, rank(short_all) - 0.5); dual_short = 0.60 * f_cov + 0.40 * f_all; vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); reversal = signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); signal = 0.65 * signed_power(dual_short, 1.5) + 0.20 * vol_burst - 0.04 * reversal; group_neutralize(signed_power(signal, 4.0), densify(subindustry))".to_string(),
                universe: "TOP1000".to_string(),
                decay: 28,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "SHORT_INTEREST".to_string(),
            },
            CandidateAlpha {
                name: "US_D1_TOP3000_Short_Accel_SubNeut_Dec22".to_string(),
                expression: "short_val = ts_backfill(vec_avg(shares_sold_short_count_2), 60); short_factor = if_else(is_nan(short_val), 0, rank(short_val) - 0.5); short_delta = ts_delta(short_val, 20); short_accel = if_else(is_nan(short_delta), 0, rank(short_delta) - 0.5); vol_factor = rank(volume / ts_mean(volume, 40)) - 0.5; signal = 0.70 * signed_power(short_factor, 1.5) + 0.30 * signed_power(short_accel, 1.3) + 0.25 * signed_power(vol_factor, 1.2); delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); signal_clean = rank(signal - delta_damp) - 0.5; group_neutralize(signed_power(signal_clean, 3.8), densify(subindustry))".to_string(),
                universe: "TOP3000".to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "SHORT_INTEREST".to_string(),
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
        Some(FactorPillar::CrossSanctuary) => vec![
            CandidateAlpha {
                name: format!("US_D1_{}_News_Short_Hybrid_SubNeut_Dec28", u),
                expression: "news_sl = ts_backfill(vec_avg(nws12_afterhsz_sl), 20); short_cov = ts_backfill(vec_avg(shares_sold_short_count_2), 40); f_news = if_else(is_nan(news_sl), 0, rank(news_sl) - 0.5); f_short = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); signal = 0.50 * signed_power(f_news, 1.5) + 0.40 * signed_power(f_short, 1.4) + 0.20 * vol_burst; delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); group_neutralize(signed_power(signal - delta_damp, 4.0), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 28,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_News_DualShort_VWAP_Dec28", u),
                expression: "news_sl = ts_backfill(vec_avg(nws12_afterhsz_sl), 20); short_cov = ts_backfill(vec_avg(shares_sold_short_count_2), 40); short_all = ts_backfill(vec_avg(shorted_shares_count_all), 40); f_news = if_else(is_nan(news_sl), 0, rank(news_sl) - 0.5); f_cov = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); f_all = if_else(is_nan(short_all), 0, rank(short_all) - 0.5); dual_short = 0.60 * f_cov + 0.40 * f_all; vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); vwap_dev = signed_power(rank(vwap / close - 1) - 0.5, 1.4); signal = 0.50 * signed_power(f_news, 1.5) + 0.40 * signed_power(dual_short, 1.4) + 0.15 * vol_burst + 0.15 * vwap_dev; delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); group_neutralize(signed_power(signal - delta_damp, 4.2), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 28,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_Short_VWAP_OrderFlow_Dec25", u),
                expression: "short_cov = ts_backfill(vec_avg(shares_sold_short_count_2), 40); f_short = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); vwap_dev = signed_power(rank(vwap / close - 1) - 0.5, 1.4); vol_burst = signed_power(rank(volume / ts_mean(volume, 30)) - 0.5, 1.2); signal = 0.60 * signed_power(f_short, 1.4) + 0.25 * vwap_dev + 0.20 * vol_burst; delta_damp = 0.04 * signed_power(rank(ts_decay_linear(ts_delta(close, 4), 3)) - 0.5, 1.5); group_neutralize(signed_power(signal - delta_damp, 4.0), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 25,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
            },
            CandidateAlpha {
                name: format!("US_D1_{}_NightDay_BorrowCost_Dec22", u),
                expression: "night_ret = winsorize((open - ts_delay(close, 1)) / ts_delay(close, 1)); day_ret = winsorize((close - open) / open); spread = ts_mean(night_ret, 10) - ts_mean(day_ret, 10); short_cov = ts_backfill(vec_avg(shares_sold_short_count_2), 40); f_short = if_else(is_nan(short_cov), 0, rank(short_cov) - 0.5); signal = 0.65 * signed_power(rank(spread) - 0.5, 1.8) + 0.35 * signed_power(f_short, 1.4); group_neutralize(signed_power(signal, 4.2), densify(subindustry))".to_string(),
                universe: u.to_string(),
                decay: 22,
                neutralization: "SUBINDUSTRY".to_string(),
                truncation: 0.07,
                pillar: "CROSS_SANCTUARY".to_string(),
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
