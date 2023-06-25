#[macro_use]
extern crate log;

use anyhow::Result;

fn main() -> Result<()> {
    pretty_env_logger::init();

    info!("Hello, world!");

    Ok(())
}
