#[macro_use]
extern crate clap;

pub mod analyzer;
pub mod config;
pub mod database;
pub mod key_prefix;
pub mod result_formatters;

use crate::config::Config;

fn main() {
    // Initialize Rustls crypto provider for TLS support
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let mut config = Config::new();
    let result = analyzer::run(&mut config);

    result_formatters::call(&config, &result);
}
