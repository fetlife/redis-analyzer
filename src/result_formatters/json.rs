use serde_json;

use super::Formatter;
use crate::analyzer::AnalyzerResult;
use crate::config::Config;

pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn call(&self, _config: &Config, result: &AnalyzerResult) {
        let json = serde_json::to_string(&result.root_prefix).unwrap();
        println!("{}", json);
    }
}
