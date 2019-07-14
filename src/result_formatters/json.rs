use serde_json;

use crate::config::Config;
use crate::prefix::Prefix;

pub fn call(_config: &Config, root_prefix: &Prefix) {
    let j = serde_json::to_string(&root_prefix).unwrap();

    println!("{}", j);
}
