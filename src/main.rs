use color_eyre::Result;

use crate::config::Config;

pub(crate) mod analyzer;
pub(crate) mod config;
pub(crate) mod database;
pub(crate) mod key_prefix;
pub(crate) mod result_formatters;

fn main() -> Result<()> {
    let mut config = Config::new()?;
    let result = analyzer::run(&mut config);

    result_formatters::get_formatter(&config).call(&config, &result);
    Ok(())
}
