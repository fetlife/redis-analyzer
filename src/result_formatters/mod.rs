pub mod json;
pub mod plain;

use crate::config::{Config, OutputFormat};
use crate::key_prefix::KeyPrefix;

pub fn call(config: &Config, root_prefix: &KeyPrefix) {
    let formatter = match config.output_format {
        OutputFormat::Plain => self::plain::call,
        OutputFormat::Json => self::json::call,
    };

    formatter(config, root_prefix);
}
