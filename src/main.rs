#[macro_use]
extern crate clap;

pub mod analyzer;
pub mod config;
pub mod database;
pub mod key_prefix;
pub mod result_formatters;

use crate::config::Config;

fn main() {
    let mut config = Config::new();
    let result = analyzer::run(&mut config);

    result_formatters::call(&config, &result);
}
