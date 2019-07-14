use serde_json;

use crate::config::Config;
use crate::key_prefix::KeyPrefix;

pub fn call(_config: &Config, root_prefix: &KeyPrefix) {
    let j = serde_json::to_string(&root_prefix).unwrap();

    println!("{}", j);
}
