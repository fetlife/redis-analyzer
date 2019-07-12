use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;
use pretty_bytes::converter::convert;
use redis;
use regex::Regex;
use std::collections::HashMap;

pub struct PrefixStats {
    pub value: Option<String>,
    pub depth: usize,
    pub count: usize,
    pub memory_usage: usize,
    pub subkeys: HashMap<String, PrefixStats>,
}

impl PrefixStats {
    pub fn new(prefix: Option<&str>, depth: usize, count: usize) -> Self {
        Self {
            value: prefix.map(|s| s.to_string()),
            depth,
            count,
            memory_usage: 0,
            subkeys: HashMap::new(),
        }
    }
}

fn main() {
    let client = redis::Client::open("redis://127.0.0.1:6379/12").expect("connect to redis");
    let mut con = client.get_connection().expect("getting connection");

    let dbsize: usize = redis::cmd("DBSIZE")
        .query(&mut con)
        .expect("getting dbsize");

    let mut top_stats = PrefixStats::new(None, 0, dbsize);

    gather_stats(&mut top_stats, dbsize, &mut con);

    println!("");

    gather_memory_usage_stats(&mut top_stats, &mut con);

    print_stats(&top_stats, top_stats.memory_usage);
}

pub fn gather_stats(prefix_stats: &mut PrefixStats, dbsize: usize, redis: &mut redis::Connection) {
    println!(
        "Scanning {}",
        prefix_stats.value.as_ref().unwrap_or(&"root".to_string()),
    );

    let delimiter = Regex::new(r"[/:]+").unwrap();

    let mut scan_command = redis::cmd("SCAN")
        .cursor_arg(0)
        .arg("COUNT")
        .arg("100")
        .clone();

    if let Some(p) = &prefix_stats.value {
        scan_command = scan_command.arg("MATCH").arg(format!("{}*", p)).clone();
    }

    let iter: redis::Iter<String> = scan_command.clone().iter(redis).expect("running scan");

    let mut frequency: HashMapFrequency<String> = HashMapFrequency::new();

    for (_i, key) in iter.enumerate() {
        // if i % 100_000 == 0 && i > 0 {
        //     println!("{}", i);
        //     // break;
        // }

        let mut delimiter_positions = delimiter.find_iter(&key);

        let prefix = match delimiter_positions.nth(prefix_stats.depth) {
            None => key,
            Some(position) => unsafe { key.get_unchecked(0..position.start()) }.to_string(),
        };

        frequency.increment(prefix);
    }

    for (prefix, count) in frequency.iter() {
        let mut subkey = PrefixStats::new(Some(prefix), prefix_stats.depth + 1, *count);

        // if key count is larger than 1% of all keys count
        if *count > dbsize / 100 {
            gather_stats(&mut subkey, dbsize, redis);
            prefix_stats.subkeys.insert(prefix.to_string(), subkey);
        } else if prefix_stats.depth == 0 && *count > 100 {
            prefix_stats.subkeys.insert(prefix.to_string(), subkey);
        }
    }
}

pub fn gather_memory_usage_stats(prefix_stats: &mut PrefixStats, redis: &mut redis::Connection) {
    let mut cursor: u64 = 0;
    let mut total = 0;
    let mut iterations = 0;
    let scan_size = 100;
    let scan_size_arg = format!("{}", scan_size);

    loop {
        if iterations % 10_000 == 0 {
            println!("memory scan: {}", iterations);
        }

        let scan_command = redis::cmd("SCAN")
            .cursor_arg(cursor)
            .arg("COUNT")
            .arg(&scan_size_arg)
            .clone();

        let (new_cursor, keys): (u64, Vec<String>) = scan_command.query(redis).expect("scan");

        cursor = new_cursor;

        let mut memory_usage_command = redis::pipe().clone();

        for key in keys.iter() {
            memory_usage_command = memory_usage_command
                .cmd("MEMORY")
                .arg("USAGE")
                .arg(key)
                .clone();
        }

        let memory_usages: Vec<usize> = memory_usage_command
            .query(redis)
            .expect("memory usage command");

        for (key, memory_usage) in keys.iter().zip(memory_usages.iter()) {
            prefix_stats.memory_usage += memory_usage;

            record_memory_usage(prefix_stats, key, *memory_usage);
        }

        total += memory_usages.iter().fold(0, |a, b| a + b);

        iterations += scan_size;

        if cursor == 0 {
            break;
        }
    }

    dbg!(total);
}

pub fn record_memory_usage(prefix_stats: &mut PrefixStats, key: &str, memory_usage: usize) {
    if prefix_stats
        .value
        .as_ref()
        .map_or(true, |prefix| key.starts_with(prefix))
    {
        prefix_stats.memory_usage += memory_usage;

        for (_, sub_prefix_stats) in prefix_stats.subkeys.iter_mut() {
            record_memory_usage(sub_prefix_stats, key, memory_usage);
        }
    }
}

pub fn print_stats(prefix_stats: &PrefixStats, parent_memory_usage: usize) {
    println!(
        "{:indent$}{} => count: {}, size: {} ({:.2}%)",
        "",
        prefix_stats.value.as_ref().unwrap_or(&"root".to_string()),
        prefix_stats.count,
        convert(prefix_stats.memory_usage as f64),
        prefix_stats.memory_usage as f32 / parent_memory_usage as f32 * 100.,
        indent = prefix_stats.depth * 2,
    );

    let mut subkeys: Vec<&PrefixStats> = prefix_stats.subkeys.values().collect();

    if subkeys.is_empty() {
        return;
    }

    subkeys.sort_by_key(|k| k.memory_usage);
    subkeys.reverse();

    let mut other = prefix_stats.count;
    let mut other_memory_usage = prefix_stats.memory_usage;

    for stats in subkeys.iter() {
        other -= stats.count;
        other_memory_usage -= stats.memory_usage;
        print_stats(stats, prefix_stats.memory_usage);
    }

    let other_percentage = other_memory_usage as f32 / prefix_stats.memory_usage as f32 * 100.;

    if other_percentage < 1. {
        return;
    }

    println!(
        "{:indent$}{} => count: {}, size: {} ({:.2}%)",
        "",
        "other",
        other,
        convert(other_memory_usage as f64),
        other_percentage,
        indent = (prefix_stats.depth + 1) * 2,
    );
}
