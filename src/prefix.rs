use std::vec::Vec;

pub struct Prefix {
    pub value: Option<String>,
    pub depth: usize,
    pub keys_count: usize,
    pub memory_usage: usize,
    pub children: Vec<Prefix>,
}

impl Prefix {
    pub fn new(prefix: Option<&str>, depth: usize, keys_count: usize, memory_usage: usize) -> Self {
        Self {
            value: prefix.map(|s| s.to_string()),
            depth,
            keys_count,
            memory_usage,
            children: Vec::new(),
        }
    }
}
