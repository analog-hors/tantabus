use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{Parser, Subcommand};

mod analyzed_game;
mod game_gen;
mod extract_positions;

use extract_positions::{ExtractPositionsConfig, run_position_extraction};
use game_gen::{GameGenRunnerConfig, run_game_gen};

#[derive(Parser)]
/// Generate and process analyzed Tantabus games. 
struct DatagenCommand {
    #[clap(subcommand)]
    subcommand: DatagenSubcommand
}

#[derive(Subcommand)]
enum DatagenSubcommand {
    GenGames(GameGenRunnerConfig),
    ExtractPos(ExtractPositionsConfig)
}

fn main() {
    let abort = Arc::new(AtomicBool::new(false));

    ctrlc::set_handler({
        let abort = Arc::clone(&abort);
        move || {
            abort.store(true, Ordering::SeqCst);
        }
    }).unwrap();

    match DatagenCommand::parse().subcommand {
        DatagenSubcommand::GenGames(config) => run_game_gen(&config, &abort),
        DatagenSubcommand::ExtractPos(config) => run_position_extraction(&config, &abort)
    }
}
