

pub struct Database {
    pub host: String,
    pub url: String,
    pub keys_count: usize,
    pub connection: redis::Connection,
}