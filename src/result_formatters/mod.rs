pub mod json;
pub mod plain;

use crate::analyzer::Result;
use crate::config::{Config, OutputFormat};

pub fn call(config: &Config, result: &Result) {
    let formatter = match config.output_format {
        OutputFormat::Plain => self::plain::call,
        OutputFormat::Json => self::json::call,
    };

    formatter(config, result);
}
