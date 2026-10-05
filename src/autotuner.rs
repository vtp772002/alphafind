use crate::client::BrainClient;
use crate::models::{AlphaDetails, AlphaSettings};
use anyhow::Result;
use colored::Colorize;

/// Automatically tunes alpha simulation parameters to maximise fitness scores.
pub struct FitnessAutoTuner<'a> {
    client: &'a BrainClient,
}

impl<'a> FitnessAutoTuner<'a> {
    /// Creates a new tuner backed by the given [`BrainClient`].
    pub fn new(client: &'a BrainClient) -> Self {
        Self { client }
    }

    /// Automatically sweeps decay parameter to push Fitness >= 1.50 (GOOD Grade)
    pub async fn tune_decay(
        &self,
        expression: &str,
        base_settings: &AlphaSettings,
        target_fitness: f64,
    ) -> Result<Option<AlphaDetails>> {
        println!(
            "{}",
            format!(
                "[*] [AutoTuner] Launching parameter sweep on {} (Target Fitness >= {:.2})...",
                base_settings.universe, target_fitness
            )
            .cyan()
        );

        let decay_steps = [
            base_settings.decay + 4,
            base_settings.decay + 8,
            base_settings.decay + 14,
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
                        "     🌟 [AutoTuner Success] Alpha {} conquered Fitness {:.2}!",
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
}
