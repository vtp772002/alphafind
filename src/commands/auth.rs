//! Authentication, portfolio synchronization, and platform status commands.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::commands::{get_main_client, json_path_display};
use crate::models::PortfolioAlpha;
use crate::screener::MultiAccountScreener;

/// Authenticates all configured BRAIN accounts and sessions concurrently.
pub async fn cmd_auth() -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Multi-Account Authentication"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let screener = MultiAccountScreener::from_env(3)?;
    screener.authenticate_all().await?;
    Ok(())
}

/// Syncs the active Out-of-Sample (OS) portfolio from BRAIN into portfolio_os.json.
pub async fn cmd_sync() -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — Out-of-Sample (OS) Portfolio Sync"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!("  Connecting to MAIN account: {}...", client.email().cyan());
    let portfolio = client.fetch_active_portfolio().await?;

    println!(
        "  {} Found {} active OS alphas.",
        "✅".green(),
        portfolio.len()
    );

    let mut univ_counts: HashMap<String, usize> = HashMap::new();
    for p in &portfolio {
        let u = p.universe.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        *univ_counts.entry(u).or_insert(0) += 1;
    }

    println!("\n  {} Universe Diversification Breakdown:", "📊".bold());
    for (u, count) in &univ_counts {
        let bar = "█".repeat(*count * 2);
        println!("    {:<12} : {:2} alphas {}", u.yellow(), count, bar.cyan());
    }

    let json_path = "portfolio_os.json";
    let json_str = serde_json::to_string_pretty(&portfolio)?;
    if let Err(e) = fs::write(json_path, json_str) {
        eprintln!("  ⚠️  Warning: Failed to write cache: {}", e);
    } else {
        println!(
            "\n  {} Successfully updated {}",
            "💾".green(),
            json_path.bold()
        );
    }

    if let Ok(competitions) = client.fetch_competitions().await {
        for comp in &competitions {
            if let Some(lb) = &comp.leaderboard {
                println!(
                    "\n  {} Live Leaderboard Status ({}):",
                    "🏆".bold(),
                    comp.name.yellow()
                );
                if let Some(r) = lb.rank {
                    println!(
                        "    Rank: #{} | Score: {:.2} | isScore: {:.1} | Uniqueness: {:.2} | Days: {}/60",
                        r.to_string().bold().green(),
                        lb.score.unwrap_or(0.0),
                        lb.is_score.unwrap_or(0.0),
                        lb.uniqueness_score.unwrap_or(0.0),
                        lb.days_of_submission.unwrap_or(0)
                    );
                }
            }
        }
    }

    Ok(())
}

/// Views live user status, rank, and official BRAIN leaderboard scores.
pub async fn cmd_score() -> Result<()> {
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );
    println!(
        "{}",
        "  AlphaFind Quant Engine — WorldQuant BRAIN Live Leaderboard & Status"
            .bold()
            .cyan()
    );
    println!(
        "{}",
        "═════════════════════════════════════════════════════════════════════════".cyan()
    );

    let client = get_main_client()?;
    println!("  Connecting to MAIN account: {}...", client.email().cyan());

    let user_profile = client.fetch_user_profile().await.ok();
    if let Some(ref profile) = user_profile {
        let name_str = profile.full_name.as_deref().unwrap_or("Quant Researcher");
        let level_str = profile.level.as_deref().unwrap_or("MEMBER");
        println!(
            "  👤 User: {} ({}) | Email: {} | Tier: {}",
            name_str.bold().white(),
            profile.id.yellow(),
            profile.email.cyan(),
            level_str.yellow().bold()
        );
    }

    let competitions = client.fetch_competitions().await?;
    let mut found_board = false;

    for comp in &competitions {
        if let Some(lb) = &comp.leaderboard {
            found_board = true;
            println!(
                "\n  🏆 Competition: {} [{}]",
                comp.name.bold().yellow(),
                comp.id.cyan()
            );
            println!("  ─────────────────────────────────────────────────────────────────────────");
            if let Some(r) = lb.rank {
                let rank_str = format!("#{}", r);
                let rank_display = if r == 1 {
                    rank_str.bold().green()
                } else if r <= 3 {
                    rank_str.bold().yellow()
                } else {
                    rank_str.bold().white()
                };
                println!("    Leaderboard Rank:          {}", rank_display);
            }
            if let Some(sc) = lb.score {
                println!(
                    "    Total Leaderboard Score:   {}",
                    format!("{:.2}", sc).bold().green()
                );
            }
            if let Some(is) = lb.is_score {
                println!(
                    "    In-Sample Score (isScore): {}",
                    format!("{:.1}", is).bold().yellow()
                );
            }
            if let Some(uniq) = lb.uniqueness_score {
                println!(
                    "    Uniqueness Score:          {}",
                    format!("{:.2}", uniq).cyan()
                );
            }
            if let Some(days) = lb.days_of_submission {
                println!(
                    "    Days of Submission:        {} / 60",
                    days.to_string().cyan()
                );
            }
            if let Some(uni) = &lb.university {
                println!("    Affiliation / University:  {}", uni.white());
            }
            println!("  ─────────────────────────────────────────────────────────────────────────");

            // Fetch Top Podium Competitors
            if let Ok(board) = client.fetch_competition_leaderboard(&comp.id, 5).await {
                println!(
                    "\n  🏅 National Podium Standings (Top {} / {} Competitors):",
                    board.results.len().min(5),
                    board.count
                );
                println!("  ┌──────┬────────────────────────┬─────────┬───────────┬────────────┬──────┬────────────────────────────┐");
                println!("  │ Rank │ Competitor             │ Score   │ isScore   │ Uniqueness │ Days │ University                 │");
                println!("  ├──────┼────────────────────────┼─────────┼───────────┼────────────┼──────┼────────────────────────────┤");

                for entry in board.results.iter().take(5) {
                    let is_me = if let Some(ref prof) = user_profile {
                        entry.user.id() == prof.id
                    } else {
                        false
                    };

                    let rank_badge = match entry.rank {
                        1 => "🥇 #1".yellow().bold(),
                        2 => "🥈 #2".bright_white().bold(),
                        3 => "🥉 #3".bright_yellow().bold(),
                        r => format!("   #{}", r).normal(),
                    };

                    let comp_name = if is_me {
                        format!("{} (YOU)", entry.user.display_name())
                            .green()
                            .bold()
                    } else {
                        entry.user.display_name().normal()
                    };

                    let uni_short = entry.university.as_deref().unwrap_or("—");
                    let uni_display = if uni_short.len() > 26 {
                        format!("{}...", &uni_short[..23])
                    } else {
                        uni_short.to_string()
                    };

                    println!(
                        "  │ {:<4} │ {:<22} │ {:>7.2} │ {:>9.1} │ {:>10.2} │ {:>4} │ {:<26} │",
                        rank_badge,
                        comp_name,
                        entry.score,
                        entry.is_score,
                        entry.uniqueness_score,
                        entry.days_of_submission,
                        uni_display
                    );
                }
                println!("  └──────┴────────────────────────┴─────────┴───────────┴────────────┴──────┴────────────────────────────┘");
            }
        }
    }

    if !found_board {
        println!("  ℹ️  No active competition leaderboard entry found.");
    }

    // Also display portfolio summary if portfolio_os.json exists
    let port_path = "portfolio_os.json";
    if Path::new(port_path).exists() {
        if let Ok(port_data) = fs::read_to_string(port_path) {
            if let Ok(os_alphas) = serde_json::from_str::<Vec<PortfolioAlpha>>(&port_data) {
                println!(
                    "\n  📊 Active OS Portfolio: {} Alphas tracked in {}",
                    os_alphas.len().to_string().bold().green(),
                    json_path_display(port_path)
                );
            }
        }
    }

    Ok(())
}
