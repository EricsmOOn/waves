use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use waves::app::{
    build_session_with_scenarios_dir, inspect_config_with_scenarios_dir, replay_run,
    run_headless_with_scenarios_dir, run_play, validate_scenario_with_scenarios_dir,
};
use waves::tui::{run_tui, run_tui_remote_with_scenarios_dir};

#[derive(Debug, Parser)]
#[command(name = "waves")]
#[command(about = "Agent autonomous decision observation framework")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate {
        #[arg(long, value_name = "DIR")]
        scenarios_dir: Option<PathBuf>,
        #[command(subcommand)]
        target: ValidateTarget,
    },
    Inspect {
        #[arg(long, value_name = "DIR")]
        scenarios_dir: Option<PathBuf>,
        #[command(subcommand)]
        target: InspectTarget,
    },
    Replay {
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value = "data/waves.sqlite")]
        db: PathBuf,
    },
    Run {
        #[arg(long, default_value = "sea_survival")]
        scenario: String,
        #[arg(long, default_value = "zh-CN")]
        locale: String,
        #[arg(long, default_value_t = 48)]
        ticks: u64,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value = "data/waves.sqlite")]
        db: PathBuf,
        #[arg(long, value_name = "DIR")]
        scenarios_dir: Option<PathBuf>,
    },
    Tui {
        #[arg(long, default_value = "sea_survival")]
        scenario: String,
        #[arg(long, default_value = "zh-CN")]
        locale: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value = "data/waves.sqlite")]
        db: PathBuf,
        #[arg(long, default_value_t = 800)]
        tick_ms: u64,
        #[arg(long)]
        connect: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        scenarios_dir: Option<PathBuf>,
    },
    Serve {
        #[arg(long, default_value = "sea_survival")]
        scenario: String,
        #[arg(long, default_value = "zh-CN")]
        locale: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value = "data/waves.sqlite")]
        db: PathBuf,
        #[arg(long, default_value = "data/waves.sock")]
        socket: PathBuf,
        #[arg(long, value_name = "DIR")]
        scenarios_dir: Option<PathBuf>,
    },
    Play {
        #[arg(long, default_value = "sea_survival")]
        scenario: String,
        #[arg(long, default_value = "zh-CN")]
        locale: String,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value = "data/waves.sqlite")]
        db: PathBuf,
        #[arg(long, default_value = "data/waves.sock")]
        socket: PathBuf,
        #[arg(long, value_name = "DIR")]
        scenarios_dir: Option<PathBuf>,
    },
    Mcp {
        #[arg(long)]
        connect: Option<PathBuf>,
        #[arg(long, value_name = "DIR")]
        scenarios_dir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ValidateTarget {
    Scenario { scenario: String },
}

#[derive(Debug, Subcommand)]
enum InspectTarget {
    Config { scenario: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate {
            scenarios_dir,
            target,
        } => match target {
            ValidateTarget::Scenario { scenario } => {
                let errors =
                    validate_scenario_with_scenarios_dir(&scenario, scenarios_dir.as_deref())?;
                if errors.is_empty() {
                    println!("scenario {scenario} is valid");
                } else {
                    for error in &errors {
                        println!("{error}");
                    }
                    std::process::exit(1);
                }
            }
        },
        Commands::Inspect {
            scenarios_dir,
            target,
        } => match target {
            InspectTarget::Config { scenario } => {
                let inspection =
                    inspect_config_with_scenarios_dir(&scenario, scenarios_dir.as_deref())?;
                for line in inspection.lines() {
                    println!("{line}");
                }
            }
        },
        Commands::Run {
            scenario,
            locale,
            ticks,
            seed,
            db,
            scenarios_dir,
        } => {
            let report = run_headless_with_scenarios_dir(
                &scenario,
                &locale,
                ticks,
                seed,
                Some(db),
                scenarios_dir.as_deref(),
            )?;
            println!("run_id: {}", report.run_id);
            println!("tick: {}", report.final_state.tick);
            println!("day: {}", report.final_state.environment.day);
            println!("decisions: {}", report.decisions);
            println!("logs: {}", report.logs);
            println!("domain_events: {}", report.domain_events);
            println!("ui_events: {}", report.ui_events);
            println!("pending_decision: {}", report.pending_decision);
            println!(
                "state: hp={:.0} thirst={:.0} energy={:.0} raft={:.0} water={:.2} food={:.2} distance={:.1}",
                report.final_state.stats.hp,
                report.final_state.stats.thirst,
                report.final_state.stats.energy,
                report.final_state.stats.raft,
                report.final_state.resources.water,
                report.final_state.resources.food,
                report.final_state.environment.distance_to_land,
            );
            if let Some(outcome) = report.final_state.outcome {
                println!("outcome: {outcome}");
            }
        }
        Commands::Tui {
            scenario,
            locale,
            seed,
            db,
            tick_ms,
            connect,
            scenarios_dir,
        } => {
            if let Some(socket_path) = connect {
                run_tui_remote_with_scenarios_dir(
                    socket_path,
                    Duration::from_millis(tick_ms),
                    scenarios_dir,
                )?;
            } else {
                let session = build_session_with_scenarios_dir(
                    &scenario,
                    &locale,
                    seed,
                    Some(db),
                    scenarios_dir.as_deref(),
                )?;
                run_tui(session, Duration::from_millis(tick_ms))?;
            }
        }
        Commands::Serve {
            scenario,
            locale,
            seed,
            db,
            socket,
            scenarios_dir,
        } => {
            println!("waves daemon listening on {}", socket.display());
            let scenarios_hint = scenarios_dir
                .as_ref()
                .map(|path| format!(" --scenarios-dir {}", path.display()))
                .unwrap_or_default();
            println!(
                "observer: cargo run -- tui --connect {}{}",
                socket.display(),
                scenarios_hint
            );
            println!(
                "mcp bridge: cargo run -- mcp --connect {}{}",
                socket.display(),
                scenarios_hint
            );
            waves::daemon::run_server_with_scenarios_dir(
                &scenario,
                &locale,
                seed,
                Some(db),
                socket,
                scenarios_dir,
            )?;
        }
        Commands::Play {
            scenario,
            locale,
            seed,
            db,
            socket,
            scenarios_dir,
        } => {
            run_play(&scenario, &locale, seed, Some(db), socket, scenarios_dir)?;
        }
        Commands::Replay { run_id, db } => {
            let summary = replay_run(db, &run_id)?;
            for line in summary.lines() {
                println!("{line}");
            }
        }
        Commands::Mcp {
            connect,
            scenarios_dir,
        } => {
            waves::mcp::run_stdio_with_scenarios_dir(connect, scenarios_dir)?;
        }
    }
    Ok(())
}
