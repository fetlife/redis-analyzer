use color_eyre::Result;

use crate::config::Config;

pub mod analyzer;
pub mod config;
pub mod database;
pub mod key_prefix;
pub mod result_formatters;


fn main() -> Result<()> {
    let mut config = Config::new()?;
    let result = analyzer::run(&mut config);

    result_formatters::call(&config, &result);
    Ok(())
}
