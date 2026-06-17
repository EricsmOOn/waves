use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use waves::app::{
    build_session, inspect_config, replay_run, run_headless, run_play, validate_scenario,
};
use waves::tui::{run_tui, run_tui_remote};

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
        #[command(subcommand)]
        target: ValidateTarget,
    },
    Inspect {
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
    },
    Mcp {
        #[arg(long)]
        connect: Option<PathBuf>,
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
        Commands::Validate { target } => match target {
            ValidateTarget::Scenario { scenario } => {
                let errors = validate_scenario(&scenario)?;
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
        Commands::Inspect { target } => match target {
            InspectTarget::Config { scenario } => {
                let inspection = inspect_config(&scenario)?;
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
        } => {
            let report = run_headless(&scenario, &locale, ticks, seed, Some(db))?;
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
        } => {
            if let Some(socket_path) = connect {
                run_tui_remote(socket_path, Duration::from_millis(tick_ms))?;
            } else {
                let session = build_session(&scenario, &locale, seed, Some(db))?;
                run_tui(session, Duration::from_millis(tick_ms))?;
            }
        }
        Commands::Serve {
            scenario,
            locale,
            seed,
            db,
            socket,
        } => {
            println!("waves daemon listening on {}", socket.display());
            println!("observer: cargo run -- tui --connect {}", socket.display());
            println!(
                "mcp bridge: cargo run -- mcp --connect {}",
                socket.display()
            );
            waves::daemon::run_server(&scenario, &locale, seed, Some(db), socket)?;
        }
        Commands::Play {
            scenario,
            locale,
            seed,
            db,
            socket,
        } => {
            run_play(&scenario, &locale, seed, Some(db), socket)?;
        }
        Commands::Replay { run_id, db } => {
            let summary = replay_run(db, &run_id)?;
            for line in summary.lines() {
                println!("{line}");
            }
        }
        Commands::Mcp { connect } => {
            waves::mcp::run_stdio(connect)?;
        }
    }
    Ok(())
}
