use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Serialize, Deserialize)]
pub struct KeyPrefix {
    pub value: String,
    pub depth: usize,
    pub keys_count: usize,
    pub memory_usage: usize,
    pub children: Vec<KeyPrefix>,
}

impl KeyPrefix {
    pub fn new(prefix: &str, depth: usize, keys_count: usize, memory_usage: usize) -> Self {
        Self {
            value: prefix.to_string(),
            depth,
            keys_count,
            memory_usage,
            children: Vec::new(),
        }
    }
}
