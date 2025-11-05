use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::config::Config;

pub(crate) mod analyzer;
pub(crate) mod config;
pub(crate) mod database;
pub(crate) mod key_prefix;
pub(crate) mod result_formatters;

fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|e| eyre!("failed to install default crypto provider: {:?}", e))?;
    let mut config = Config::new()?;
    let result = analyzer::run(&mut config);

    result_formatters::get_formatter(&config).call(&config, &result);
    Ok(())
}
