use crate::client::BrainClient;
use crate::config::quant::{
    min_fitness_for_delay, min_sharpe_for_delay, turnover_within_platform_bounds,
};
use crate::models::{
    AlphaDetails, AlphaSettings, RobustnessReport, RobustnessVerdict, SensitivityPoint,
    TunedAlphaResult,
};
use anyhow::Result;
use colored::Colorize;

/// Evaluates parameter sensitivity points to determine if the peak is a robust plateau or an overfitted needle peak.
pub fn evaluate_robustness(
    mut points: Vec<SensitivityPoint>,
    param_name: &str,
    min_acceptable_sharpe: f64,
    max_drop_threshold_pct: f64,
) -> RobustnessReport {
    points.sort_by(|a, b| {
        a.param_value
            .partial_cmp(&b.param_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if points.len() < 3 {
        let opt_val = points.first().map(|p| p.param_value).unwrap_or(0.0);
        let opt_sh = points.first().map(|p| p.sharpe).unwrap_or(0.0);
        let opt_fit = points.first().map(|p| p.fitness).unwrap_or(0.0);
        return RobustnessReport {
            param_name: param_name.to_string(),
            optimal_value: opt_val,
            optimal_sharpe: opt_sh,
            optimal_fitness: opt_fit,
            points,
            max_neighbor_drop_pct: 0.0,
            sharpe_cv: 0.0,
            min_neighbor_sharpe: 0.0,
            verdict: RobustnessVerdict::Inconclusive,
            summary: "Insufficient sweep points (minimum 3 required for neighbor sensitivity)".to_string(),
        };
    }

    // Find the point with the highest Sharpe ratio
    let mut opt_idx = 0;
    let mut max_sharpe = points[0].sharpe;
    for (i, p) in points.iter().enumerate() {
        if p.sharpe > max_sharpe {
            max_sharpe = p.sharpe;
            opt_idx = i;
        }
    }

    let opt_point = &points[opt_idx];
    let opt_val = opt_point.param_value;
    let opt_sh = opt_point.sharpe;
    let opt_fit = opt_point.fitness;

    // Check adjacent neighbors
    let mut drop_left = 0.0;
    let mut drop_right = 0.0;
    let mut neighbor_sharpes = Vec::new();

    if opt_idx > 0 {
        let left_sh = points[opt_idx - 1].sharpe;
        neighbor_sharpes.push(left_sh);
        if opt_sh > 0.0 {
            drop_left = ((opt_sh - left_sh) / opt_sh).max(0.0) * 100.0;
        }
    }
    if opt_idx + 1 < points.len() {
        let right_sh = points[opt_idx + 1].sharpe;
        neighbor_sharpes.push(right_sh);
        if opt_sh > 0.0 {
            drop_right = ((opt_sh - right_sh) / opt_sh).max(0.0) * 100.0;
        }
    }

    let max_drop = drop_left.max(drop_right);
    let min_neighbor_sh = neighbor_sharpes
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);

    // Compute coefficient of variation (CV) of Sharpe across points
    let n = points.len() as f64;
    let mean_sh = points.iter().map(|p| p.sharpe).sum::<f64>() / n;
    let variance = points.iter().map(|p| (p.sharpe - mean_sh).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    let cv = if mean_sh > 0.0 { std_dev / mean_sh } else { 0.0 };

    // Verdict logic
    let is_overfit = max_drop > max_drop_threshold_pct
        || (opt_sh >= min_acceptable_sharpe && min_neighbor_sh < min_acceptable_sharpe * 0.75);

    let (verdict, summary) = if opt_sh < min_acceptable_sharpe {
        (
            RobustnessVerdict::Inconclusive,
            format!(
                "Peak Sharpe ({:.2}) fails minimum delay requirement ({:.2}).",
                opt_sh, min_acceptable_sharpe
            ),
        )
    } else if is_overfit {
        (
            RobustnessVerdict::OverfitNeedlePeak,
            format!(
                "OVERFIT: Sharpe spikes to {:.2} at {} = {:.2} but collapses by {:.1}% at adjacent neighbors (threshold: {:.1}%).",
                opt_sh, param_name, opt_val, max_drop, max_drop_threshold_pct
            ),
        )
    } else {
        (
            RobustnessVerdict::RobustPlateau,
            format!(
                "ROBUST: Smooth plateau around {} = {:.2} (Peak Sharpe: {:.2}, Max neighbor drop: {:.1}% <= {:.1}%). Hypothesis is structurally sound.",
                param_name, opt_val, opt_sh, max_drop, max_drop_threshold_pct
            ),
        )
    };

    RobustnessReport {
        param_name: param_name.to_string(),
        optimal_value: opt_val,
        optimal_sharpe: opt_sh,
        optimal_fitness: opt_fit,
        points,
        max_neighbor_drop_pct: max_drop,
        sharpe_cv: cv,
        min_neighbor_sharpe: if min_neighbor_sh.is_finite() { min_neighbor_sh } else { 0.0 },
        verdict,
        summary,
    }
}

/// Modifies the outermost/last signed_power exponent in a FASTEXPR formula
pub fn replace_last_signed_power_exponent(expr: &str, new_p: f64) -> Option<String> {
    let target = "signed_power(";
    let idx = expr.rfind(target)?;
    let after_target = &expr[idx + target.len()..];

    let comma_idx = after_target.find(',')?;
    let paren_idx = after_target[comma_idx..].find(')')? + comma_idx;

    let old_exp_str = after_target[comma_idx + 1..paren_idx].trim();
    if old_exp_str.parse::<f64>().is_ok() {
        let mut new_expr = String::new();
        new_expr.push_str(&expr[..idx + target.len() + comma_idx + 1]);
        new_expr.push_str(&format!(" {:.1}", new_p));
        new_expr.push_str(&after_target[paren_idx..]);
        Some(new_expr)
    } else {
        None
    }
}

/// Switches neutralization group in both settings and FASTEXPR densify() syntax
pub fn switch_neutralization_in_expr(expr: &str, new_neut: &str) -> String {
    let neut_upper = new_neut.to_uppercase();
    match neut_upper.as_str() {
        "SUBINDUSTRY" => expr
            .replace("densify(market)", "densify(subindustry)")
            .replace("densify(sector)", "densify(subindustry)"),
        "MARKET" => expr
            .replace("densify(subindustry)", "densify(market)")
            .replace("densify(sector)", "densify(market)"),
        "SECTOR" => expr
            .replace("densify(subindustry)", "densify(sector)")
            .replace("densify(market)", "densify(sector)"),
        _ => expr.to_string(),
    }
}

/// Multi-dimensional parameter autotuner to conquer BRAIN Fitness thresholds
pub struct FitnessAutoTuner<'a> {
    client: &'a BrainClient,
}

impl<'a> FitnessAutoTuner<'a> {
    /// Creates a new tuner backed by the given [`BrainClient`].
    pub fn new(client: &'a BrainClient) -> Self {
        Self { client }
    }

    /// Automatically sweeps decay parameter to push Fitness >= target_fitness
    pub async fn tune_decay(
        &self,
        expression: &str,
        base_settings: &AlphaSettings,
        target_fitness: f64,
    ) -> Result<Option<AlphaDetails>> {
        println!(
            "{}",
            format!(
                "[*] [AutoTuner: Decay Sweep] Launching sweep on {} (Target Fitness >= {:.2})...",
                base_settings.universe, target_fitness
            )
            .cyan()
        );

        let decay_steps = [
            base_settings.decay + 4,
            base_settings.decay + 8,
            base_settings.decay + 14,
            base_settings.decay + 20,
        ];

        for &dec in &decay_steps {
            if dec > 50 {
                continue;
            }

            let mut test_settings = base_settings.clone();
            test_settings.decay = dec;

            println!("  -> Testing Decay {}...", dec);

            let sim_id = match self
                .client
                .submit_simulation(expression, &test_settings)
                .await
            {
                Ok(id) => id,
                Err(_) => continue,
            };

            let poll_res = match self.client.poll_simulation(&sim_id, 300, 5).await {
                Ok(r) => r,
                Err(_) => continue,
            };

            let alpha_id = match poll_res.alpha {
                Some(id) => id,
                None => continue,
            };

            let details = match self.client.get_alpha_details(&alpha_id).await {
                Ok(d) => d,
                Err(_) => continue,
            };

            let st = match details.is {
                Some(ref s) => s,
                None => continue,
            };

            let sh = st.sharpe.unwrap_or(0.0);
            let fit = st.fitness.unwrap_or(0.0);
            let ret = st.returns.unwrap_or(0.0);
            let turn = st.turnover.unwrap_or(0.0);

            println!(
                "     Decay {} -> [{}] Sharpe: {:.2} | Fit: {:.2} | Ret: {:.1}% | Turn: {:.1}%",
                dec,
                alpha_id,
                sh,
                fit,
                ret * 100.0,
                turn * 100.0
            );

            let effective_target_fitness =
                target_fitness.max(min_fitness_for_delay(test_settings.delay));
            let min_sharpe = min_sharpe_for_delay(test_settings.delay);
            if fit >= effective_target_fitness
                && sh >= min_sharpe
                && turnover_within_platform_bounds(turn)
            {
                // Neighbor plateau check: ensure this is not an overfitted needle-peak spike
                let neighbor_decay = if dec > 4 { dec - 2 } else { dec + 2 };
                let mut neighbor_settings = test_settings.clone();
                neighbor_settings.decay = neighbor_decay;
                println!(
                    "     🔎 [Plateau Verification] Checking neighbor decay {}...",
                    neighbor_decay
                );

                let mut is_overfit = false;
                if let Ok(n_sim_id) = self
                    .client
                    .submit_simulation(expression, &neighbor_settings)
                    .await
                {
                    if let Ok(n_poll) = self.client.poll_simulation(&n_sim_id, 300, 5).await {
                        if let Some(n_aid) = n_poll.alpha {
                            if let Ok(n_details) = self.client.get_alpha_details(&n_aid).await {
                                if let Some(n_st) = n_details.is {
                                    let n_sh = n_st.sharpe.unwrap_or(0.0);
                                    let drop = if sh > 0.0 {
                                        ((sh - n_sh) / sh).max(0.0) * 100.0
                                    } else {
                                        0.0
                                    };
                                    println!(
                                        "        Neighbor Decay {} -> [{}] Sharpe: {:.2} (Drop: {:.1}%)",
                                        neighbor_decay, n_aid, n_sh, drop
                                    );
                                    if drop > 30.0 || n_sh < min_sharpe * 0.75 {
                                        println!(
                                            "{}",
                                            format!(
                                                "     ⚠️ [Sensitivity Warning] Decay {} is a needle-peak spike! Neighbor dropped by {:.1}%. Rejecting overfit.",
                                                dec, drop
                                            )
                                            .yellow()
                                        );
                                        is_overfit = true;
                                    }
                                }
                            }
                        }
                    }
                }

                if !is_overfit {
                    println!(
                        "{}",
                        format!(
                            "     🌟 [AutoTuner Success] Alpha {} reached Fitness {:.2} on robust decay {} (Plateau confirmed)!",
                            alpha_id, fit, dec
                        )
                        .green()
                        .bold()
                    );
                    return Ok(Some(details));
                }
            }
        }

        Ok(None)
    }

    /// Automatically sweeps non-linear convexity exponent P in signed_power(..., P)
    pub async fn tune_exponent(
        &self,
        expression: &str,
        base_settings: &AlphaSettings,
        target_fitness: f64,
    ) -> Result<Option<(AlphaDetails, String)>> {
        println!(
            "{}",
            format!(
                "[*] [AutoTuner: Exponent Sweep] Sweeping non-linear convexity P in [3.8..5.0] on {}...",
                base_settings.universe
            )
            .cyan()
        );

        let exponents = [3.8, 4.2, 4.4, 4.8, 5.0];

        for &p in &exponents {
            let mod_expr = match replace_last_signed_power_exponent(expression, p) {
                Some(e) => e,
                None => break, // Expression does not contain signed_power
            };

            println!("  -> Testing Exponent P = {:.1}...", p);

            let sim_id = match self
                .client
                .submit_simulation(&mod_expr, base_settings)
                .await
            {
                Ok(id) => id,
                Err(_) => continue,
            };

            let poll_res = match self.client.poll_simulation(&sim_id, 300, 5).await {
                Ok(r) => r,
                Err(_) => continue,
            };

            let alpha_id = match poll_res.alpha {
                Some(id) => id,
                None => continue,
            };

            let details = match self.client.get_alpha_details(&alpha_id).await {
                Ok(d) => d,
                Err(_) => continue,
            };

            let st = match details.is {
                Some(ref s) => s,
                None => continue,
            };

            let sh = st.sharpe.unwrap_or(0.0);
            let fit = st.fitness.unwrap_or(0.0);
            let ret = st.returns.unwrap_or(0.0);
            let turn = st.turnover.unwrap_or(0.0);

            println!(
                "     Exponent {:.1} -> [{}] Sharpe: {:.2} | Fit: {:.2} | Ret: {:.1}% | Turn: {:.1}%",
                p,
                alpha_id,
                sh,
                fit,
                ret * 100.0,
                turn * 100.0
            );

            let effective_target_fitness =
                target_fitness.max(min_fitness_for_delay(base_settings.delay));
            let min_sharpe = min_sharpe_for_delay(base_settings.delay);
            if fit >= effective_target_fitness
                && sh >= min_sharpe
                && turnover_within_platform_bounds(turn)
            {
                // Neighbor plateau check: ensure exponent is not a needle-peak spike
                let neighbor_p = if p > 4.0 { p - 0.4 } else { p + 0.4 };
                let mut is_overfit = false;
                if let Some(n_expr) = replace_last_signed_power_exponent(expression, neighbor_p) {
                    println!(
                        "     🔎 [Plateau Verification] Checking neighbor exponent {:.1}...",
                        neighbor_p
                    );
                    if let Ok(n_sim_id) = self
                        .client
                        .submit_simulation(&n_expr, base_settings)
                        .await
                    {
                        if let Ok(n_poll) = self.client.poll_simulation(&n_sim_id, 300, 5).await {
                            if let Some(n_aid) = n_poll.alpha {
                                if let Ok(n_details) = self.client.get_alpha_details(&n_aid).await {
                                    if let Some(n_st) = n_details.is {
                                        let n_sh = n_st.sharpe.unwrap_or(0.0);
                                        let drop = if sh > 0.0 {
                                            ((sh - n_sh) / sh).max(0.0) * 100.0
                                        } else {
                                            0.0
                                        };
                                        println!(
                                            "        Neighbor Exponent {:.1} -> [{}] Sharpe: {:.2} (Drop: {:.1}%)",
                                            neighbor_p, n_aid, n_sh, drop
                                        );
                                        if drop > 30.0 || n_sh < min_sharpe * 0.75 {
                                            println!(
                                                "{}",
                                                format!(
                                                    "     ⚠️ [Sensitivity Warning] Exponent {:.1} is a needle-peak spike! Neighbor dropped by {:.1}%. Rejecting overfit.",
                                                    p, drop
                                                )
                                                .yellow()
                                            );
                                            is_overfit = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !is_overfit {
                    println!(
                        "{}",
                        format!(
                            "     🌟 [AutoTuner Success] Exponent {:.1} reached Fitness {:.2} on {} (Plateau confirmed)!",
                            p, fit, alpha_id
                        )
                        .green()
                        .bold()
                    );
                    return Ok(Some((details, mod_expr)));
                }
            }
        }

        Ok(None)
    }

    /// Full Multi-Dimensional Hyperparameter Tuning (Decay x Exponent x Neutralization)
    pub async fn tune_hyperparameters(
        &self,
        expression: &str,
        base_settings: &AlphaSettings,
        target_fitness: f64,
    ) -> Result<Option<TunedAlphaResult>> {
        println!(
            "{}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );
        println!(
            "{}",
            format!(
                "  AlphaFind Multi-Dimensional AutoTuner [Target Fitness >= {:.2}]",
                target_fitness
            )
            .bold()
            .cyan()
        );
        println!(
            "{}",
            "═════════════════════════════════════════════════════════════════════════".cyan()
        );

        // 1. First test baseline
        println!("  Evaluating baseline simulation...");
        let base_sim_id = self
            .client
            .submit_simulation(expression, base_settings)
            .await?;
        let base_poll = self.client.poll_simulation(&base_sim_id, 350, 5).await?;
        let base_aid = match base_poll.alpha {
            Some(id) => id,
            None => anyhow::bail!("Baseline simulation failed to produce alpha ID"),
        };
        let base_details = self.client.get_alpha_details(&base_aid).await?;
        let base_st = base_details.is.clone().unwrap_or_default();
        let base_fit = base_st.fitness.unwrap_or(0.0);
        let base_sh = base_st.sharpe.unwrap_or(0.0);
        let base_ret = base_st.returns.unwrap_or(0.0);
        let base_turn = base_st.turnover.unwrap_or(0.0);

        println!(
            "  Baseline [{}] -> Sharpe: {:.2} | Fit: {:.2} | Ret: {:.1}% | Turn: {:.1}%",
            base_aid.yellow(),
            base_sh,
            base_fit,
            base_ret * 100.0,
            base_turn * 100.0
        );

        let effective_target_fitness =
            target_fitness.max(min_fitness_for_delay(base_settings.delay));
        let min_sharpe = min_sharpe_for_delay(base_settings.delay);
        if base_fit >= effective_target_fitness
            && base_sh >= min_sharpe
            && turnover_within_platform_bounds(base_turn)
        {
            println!(
                "  Baseline already satisfies target fitness {:.2}!",
                effective_target_fitness
            );
            return Ok(Some(TunedAlphaResult {
                details: base_details,
                expression: expression.to_string(),
                settings: base_settings.clone(),
                original_fitness: base_fit,
                tuned_fitness: base_fit,
                tuning_strategy: "BASELINE".to_string(),
            }));
        }

        // 2. Strategy A: Decay Parameter Sweep
        println!("\n  [Phase 1] Executing Decay Parameter Sweep...");
        if let Ok(Some(tuned_decay_alpha)) = self
            .tune_decay(expression, base_settings, target_fitness)
            .await
        {
            let t_fit = tuned_decay_alpha
                .is
                .as_ref()
                .and_then(|s| s.fitness)
                .unwrap_or(0.0);
            let sett = tuned_decay_alpha
                .settings
                .clone()
                .unwrap_or_else(|| base_settings.clone());
            return Ok(Some(TunedAlphaResult {
                details: tuned_decay_alpha,
                expression: expression.to_string(),
                settings: sett,
                original_fitness: base_fit,
                tuned_fitness: t_fit,
                tuning_strategy: "DECAY_SWEEP".to_string(),
            }));
        }

        // 3. Strategy B: Non-linear Exponent Sweep
        println!("\n  [Phase 2] Executing Non-linear Exponent Convexity Sweep...");
        if let Ok(Some((tuned_exp_alpha, mod_expr))) = self
            .tune_exponent(expression, base_settings, target_fitness)
            .await
        {
            let t_fit = tuned_exp_alpha
                .is
                .as_ref()
                .and_then(|s| s.fitness)
                .unwrap_or(0.0);
            let sett = tuned_exp_alpha
                .settings
                .clone()
                .unwrap_or_else(|| base_settings.clone());
            return Ok(Some(TunedAlphaResult {
                details: tuned_exp_alpha,
                expression: mod_expr,
                settings: sett,
                original_fitness: base_fit,
                tuned_fitness: t_fit,
                tuning_strategy: "EXPONENT_SWEEP".to_string(),
            }));
        }

        // 4. Strategy C: Neutralization Rotation
        let alt_neut = if base_settings.neutralization == "SUBINDUSTRY" {
            "MARKET"
        } else {
            "SUBINDUSTRY"
        };
        println!(
            "\n  [Phase 3] Testing Alternative Neutralization: {} -> {}...",
            base_settings.neutralization, alt_neut
        );
        let mut neut_settings = base_settings.clone();
        neut_settings.neutralization = alt_neut.to_string();
        let neut_expr = switch_neutralization_in_expr(expression, alt_neut);

        if let Ok(sim_id) = self
            .client
            .submit_simulation(&neut_expr, &neut_settings)
            .await
        {
            if let Ok(poll) = self.client.poll_simulation(&sim_id, 300, 5).await {
                if let Some(aid) = poll.alpha {
                    if let Ok(details) = self.client.get_alpha_details(&aid).await {
                        if let Some(ref st) = details.is {
                            let fit = st.fitness.unwrap_or(0.0);
                            let sh = st.sharpe.unwrap_or(0.0);
                            let turn = st.turnover.unwrap_or(0.0);
                            println!(
                                "     Neutralization {} -> [{}] Sharpe: {:.2} | Fit: {:.2}",
                                alt_neut, aid, sh, fit
                            );
                            if fit >= effective_target_fitness
                                && sh >= min_sharpe
                                && turnover_within_platform_bounds(turn)
                            {
                                return Ok(Some(TunedAlphaResult {
                                    details,
                                    expression: neut_expr,
                                    settings: neut_settings,
                                    original_fitness: base_fit,
                                    tuned_fitness: fit,
                                    tuning_strategy: format!("NEUTRALIZATION_{}", alt_neut),
                                }));
                            }
                        }
                    }
                }
            }
        }

        // 5. Strategy D: Combined Optimal Decay + Exponent 4.4
        println!(
            "\n  [Phase 4] Testing Combined Hyperparameter Matrix (Decay +10 x Exponent 4.4)..."
        );
        let mut comb_settings = base_settings.clone();
        comb_settings.decay = (base_settings.decay + 10).min(50);
        if let Some(comb_expr) = replace_last_signed_power_exponent(expression, 4.4) {
            if let Ok(sim_id) = self
                .client
                .submit_simulation(&comb_expr, &comb_settings)
                .await
            {
                if let Ok(poll) = self.client.poll_simulation(&sim_id, 300, 5).await {
                    if let Some(aid) = poll.alpha {
                        if let Ok(details) = self.client.get_alpha_details(&aid).await {
                            if let Some(ref st) = details.is {
                                let fit = st.fitness.unwrap_or(0.0);
                                let sh = st.sharpe.unwrap_or(0.0);
                                let turn = st.turnover.unwrap_or(0.0);
                                println!(
                                    "     Combined -> [{}] Sharpe: {:.2} | Fit: {:.2}",
                                    aid, sh, fit
                                );
                                if fit >= effective_target_fitness
                                    && sh >= min_sharpe
                                    && turnover_within_platform_bounds(turn)
                                {
                                    return Ok(Some(TunedAlphaResult {
                                        details,
                                        expression: comb_expr,
                                        settings: comb_settings,
                                        original_fitness: base_fit,
                                        tuned_fitness: fit,
                                        tuning_strategy: "COMBINED_DECAY_EXPONENT".to_string(),
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        println!(
            "  ⚠️ AutoTuner exhausted all search dimensions without reaching Fitness {:.2}.",
            target_fitness
        );
        Ok(None)
    }

    /// Performs a sensitivity sweep over a parameter (decay, exponent, truncation)
    /// to detect parameter overfitting (Plateau vs Needle-Peak Spike).
    pub async fn sweep_sensitivity(
        &self,
        expression: &str,
        base_settings: &AlphaSettings,
        param_type: &str,
        step_opt: Option<f64>,
        radius: usize,
    ) -> Result<RobustnessReport> {
        let param_type_lower = param_type.to_lowercase();
        let mut points = Vec::new();

        match param_type_lower.as_str() {
            "decay" => {
                let step = step_opt.unwrap_or(2.0).round() as i32;
                let step = step.max(1);
                let center = base_settings.decay;
                let r = radius as i32;

                println!(
                    "{}",
                    format!(
                        "[*] [Sensitivity Sweep: Decay] Sweeping decay in [{}..{}] (step {}) on {}...",
                        (center - r * step).max(1),
                        (center + r * step).min(50),
                        step,
                        base_settings.universe
                    )
                    .cyan()
                );

                for offset in -r..=r {
                    let d = center + offset * step;
                    if d < 1 || d > 50 {
                        continue;
                    }
                    let mut s = base_settings.clone();
                    s.decay = d;

                    let sim_id = match self.client.submit_simulation(expression, &s).await {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("    Simulation dispatch error at decay {}: {}", d, e);
                            continue;
                        }
                    };
                    if let Ok(poll) = self.client.poll_simulation(&sim_id, 300, 5).await {
                        if let Some(aid) = poll.alpha {
                            if let Ok(details) = self.client.get_alpha_details(&aid).await {
                                if let Some(st) = details.is {
                                    let sh = st.sharpe.unwrap_or(0.0);
                                    let fit = st.fitness.unwrap_or(0.0);
                                    let ret = st.returns.unwrap_or(0.0);
                                    let turn = st.turnover.unwrap_or(0.0);
                                    let margin = st.margin.unwrap_or(0.0);

                                    println!(
                                        "  Decay {:>2} -> [{}] Sharpe: {:.2} | Fit: {:.2} | Ret: {:>5.1}% | Turn: {:>5.1}%",
                                        d, aid, sh, fit, ret * 100.0, turn * 100.0
                                    );

                                    points.push(SensitivityPoint {
                                        param_name: "decay".to_string(),
                                        param_value: d as f64,
                                        alpha_id: Some(aid),
                                        sharpe: sh,
                                        fitness: fit,
                                        returns: ret,
                                        turnover: turn,
                                        margin,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "exponent" | "power" => {
                let step = step_opt.unwrap_or(0.4);
                let center = 2.0;
                let r = radius as i32;

                println!(
                    "{}",
                    format!(
                        "[*] [Sensitivity Sweep: Exponent] Sweeping signed_power exponent around {:.1} on {}...",
                        center,
                        base_settings.universe
                    )
                    .cyan()
                );

                for offset in -r..=r {
                    let p = center + offset as f64 * step;
                    if p <= 0.2 || p > 6.0 {
                        continue;
                    }
                    let mod_expr = match replace_last_signed_power_exponent(expression, p) {
                        Some(e) => e,
                        None => {
                            anyhow::bail!("Expression does not contain signed_power operator for exponent sweep");
                        }
                    };

                    let sim_id = match self.client.submit_simulation(&mod_expr, base_settings).await {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("    Simulation dispatch error at exponent {:.2}: {}", p, e);
                            continue;
                        }
                    };
                    if let Ok(poll) = self.client.poll_simulation(&sim_id, 300, 5).await {
                        if let Some(aid) = poll.alpha {
                            if let Ok(details) = self.client.get_alpha_details(&aid).await {
                                if let Some(st) = details.is {
                                    let sh = st.sharpe.unwrap_or(0.0);
                                    let fit = st.fitness.unwrap_or(0.0);
                                    let ret = st.returns.unwrap_or(0.0);
                                    let turn = st.turnover.unwrap_or(0.0);
                                    let margin = st.margin.unwrap_or(0.0);

                                    println!(
                                        "  Exp {:>4.1} -> [{}] Sharpe: {:.2} | Fit: {:.2} | Ret: {:>5.1}% | Turn: {:>5.1}%",
                                        p, aid, sh, fit, ret * 100.0, turn * 100.0
                                    );

                                    points.push(SensitivityPoint {
                                        param_name: "exponent".to_string(),
                                        param_value: p,
                                        alpha_id: Some(aid),
                                        sharpe: sh,
                                        fitness: fit,
                                        returns: ret,
                                        turnover: turn,
                                        margin,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "truncation" | "trunc" => {
                let step = step_opt.unwrap_or(0.02);
                let center = base_settings.truncation;
                let r = radius as i32;

                println!(
                    "{}",
                    format!(
                        "[*] [Sensitivity Sweep: Truncation] Sweeping weight truncation around {:.3} on {}...",
                        center, base_settings.universe
                    )
                    .cyan()
                );

                for offset in -r..=r {
                    let tr = center + offset as f64 * step;
                    if tr <= 0.005 || tr > 0.15 {
                        continue;
                    }
                    let mut s = base_settings.clone();
                    s.truncation = tr;

                    let sim_id = match self.client.submit_simulation(expression, &s).await {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("    Simulation dispatch error at truncation {:.3}: {}", tr, e);
                            continue;
                        }
                    };
                    if let Ok(poll) = self.client.poll_simulation(&sim_id, 300, 5).await {
                        if let Some(aid) = poll.alpha {
                            if let Ok(details) = self.client.get_alpha_details(&aid).await {
                                if let Some(st) = details.is {
                                    let sh = st.sharpe.unwrap_or(0.0);
                                    let fit = st.fitness.unwrap_or(0.0);
                                    let ret = st.returns.unwrap_or(0.0);
                                    let turn = st.turnover.unwrap_or(0.0);
                                    let margin = st.margin.unwrap_or(0.0);

                                    println!(
                                        "  Trunc {:>5.3} -> [{}] Sharpe: {:.2} | Fit: {:.2} | Ret: {:>5.1}% | Turn: {:>5.1}%",
                                        tr, aid, sh, fit, ret * 100.0, turn * 100.0
                                    );

                                    points.push(SensitivityPoint {
                                        param_name: "truncation".to_string(),
                                        param_value: tr,
                                        alpha_id: Some(aid),
                                        sharpe: sh,
                                        fitness: fit,
                                        returns: ret,
                                        turnover: turn,
                                        margin,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => anyhow::bail!("Unsupported parameter for sensitivity sweep: '{}'. Use 'decay', 'exponent', or 'truncation'.", param_type),
        }

        let min_sharpe = min_sharpe_for_delay(base_settings.delay);
        let report = evaluate_robustness(points, param_type, min_sharpe, 25.0);
        Ok(report)
    }
}
