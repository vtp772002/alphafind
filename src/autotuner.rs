use crate::client::BrainClient;
use crate::models::{AlphaDetails, AlphaSettings, TunedAlphaResult};
use anyhow::Result;
use colored::Colorize;

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

            if fit >= target_fitness && sh >= 1.25 && (0.01..=0.70).contains(&turn) {
                println!(
                    "{}",
                    format!(
                        "     🌟 [AutoTuner Success] Alpha {} reached Fitness {:.2}!",
                        alpha_id, fit
                    )
                    .green()
                    .bold()
                );
                return Ok(Some(details));
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

            let sim_id = match self.client.submit_simulation(&mod_expr, base_settings).await {
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

            if fit >= target_fitness && sh >= 1.25 && (0.01..=0.70).contains(&turn) {
                println!(
                    "{}",
                    format!(
                        "     🌟 [AutoTuner Success] Exponent {:.1} reached Fitness {:.2} on {}!",
                        p, fit, alpha_id
                    )
                    .green()
                    .bold()
                );
                return Ok(Some((details, mod_expr)));
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
        let base_sim_id = self.client.submit_simulation(expression, base_settings).await?;
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

        if base_fit >= target_fitness && base_sh >= 1.25 && (0.01..=0.70).contains(&base_turn) {
            println!(
                "  Baseline already satisfies target fitness {:.2}!",
                target_fitness
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
                            if fit >= target_fitness && sh >= 1.25 && (0.01..=0.70).contains(&turn) {
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
        println!("\n  [Phase 4] Testing Combined Hyperparameter Matrix (Decay +10 x Exponent 4.4)...");
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
                                if fit >= target_fitness && sh >= 1.25 && (0.01..=0.70).contains(&turn) {
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

        println!("  ⚠️ AutoTuner exhausted all search dimensions without reaching Fitness {:.2}.", target_fitness);
        Ok(None)
    }
}
