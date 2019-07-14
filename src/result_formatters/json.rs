use serde_json;

use crate::analyzer::Result;
use crate::config::Config;

pub fn call(_config: &Config, result: &Result) {
    let json = serde_json::to_string(&result.root_prefix).unwrap();

    println!("{}", json);
}
