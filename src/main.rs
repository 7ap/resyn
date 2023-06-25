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

    /// Delay in milliseconds to wait for silence before recognizing speech
    #[arg(short, long, default_value = "100")]
    delay: u32,

    /// Minimum length of time in milliseconds to oversample input audio,
    /// recommended if you speak fast in intervals under 1 second, can produce
    /// inaccurate results if set too low as it is slowing down your speech to
    /// match the model's sample rate, 500-750 is a good starting point
    #[arg(short, long, value_parser = clap::value_parser!(u32).range(0..1000))]
    interpolate: Option<u32>,
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
