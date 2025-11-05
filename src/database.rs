pub struct Database {
    pub keys_count: usize,
    pub connection: redis::Connection,
}
