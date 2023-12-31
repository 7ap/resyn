#![feature(lazy_cell)]

#[macro_use]
extern crate log;

mod recognition;
mod synthesis;

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::LazyLock;
use std::thread;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};

static CLI: LazyLock<Cli> = LazyLock::new(|| Cli::parse());

#[derive(Parser)]
pub struct Cli {
    #[clap(flatten)]
    verbosity: Verbosity<InfoLevel>,

    /// Path to the model to use for speech recognition
    model_path: PathBuf,

    /// Speaker ID to use for speech synthesis
    speaker_id: String,
}

fn main() {
    pretty_env_logger::formatted_builder()
        .filter_module("resyn", CLI.verbosity.log_level_filter())
        .init();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || recognition::run(tx));
    thread::spawn(move || synthesis::run(rx));

    loop {}
}
