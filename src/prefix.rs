use std::collections::HashMap;

pub struct Prefix {
    pub value: Option<String>,
    pub depth: usize,
    pub keys_count: usize,
    pub memory_usage: usize,
    pub children: HashMap<String, Prefix>,
}

impl Prefix {
    pub fn new(prefix: Option<&str>, depth: usize, keys_count: usize) -> Self {
        Self {
            value: prefix.map(|s| s.to_string()),
            depth,
            keys_count,
            memory_usage: 0,
            children: HashMap::new(),
        }
    }
}
