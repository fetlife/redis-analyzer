pub mod json;
pub mod plain;

use crate::analyzer::AnalyzerResult;
use crate::config::{Config, OutputFormat};

pub trait Formatter {
    fn call(&self, config: &Config, result: &AnalyzerResult);
}

pub fn get_formatter(config: &Config) -> Box<dyn Formatter> {
    match config.output_format {
        OutputFormat::Plain => Box::new(plain::PlainFormatter),
        OutputFormat::Json => Box::new(json::JsonFormatter),
    }
}
