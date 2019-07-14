pub mod json;
pub mod plain;

use crate::config::{Config, OutputFormat};
use crate::prefix::Prefix;

pub fn call(config: &Config, root_prefix: &Prefix) {
    let formatter = match config.output_format {
        OutputFormat::Plain => self::plain::call,
        OutputFormat::Json => self::json::call,
    };

    formatter(config, root_prefix);
}
