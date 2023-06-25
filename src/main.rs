#[macro_use]
extern crate log;

mod recognition;
mod synthesis;

use std::sync::mpsc;
use std::thread;

fn main() {
    pretty_env_logger::init();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || recognition::run(tx));
    thread::spawn(move || synthesis::run(rx));

    loop {}
}
