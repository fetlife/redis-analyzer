use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;
use redis;
use regex::Regex;
use std::collections::HashMap;

pub struct PrefixStats {
    pub value: Option<String>,
    pub depth: usize,
    pub count: usize,
    pub subkeys: HashMap<String, PrefixStats>,
}

impl PrefixStats {
    pub fn new(prefix: Option<&str>, depth: usize, count: usize) -> Self {
        Self {
            value: prefix.map(|s| s.to_string()),
            depth,
            count,
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

    print_stats(&top_stats, dbsize);
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
        .arg("1000")
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

pub fn print_stats(prefix_stats: &PrefixStats, parent_count: usize) {
    println!(
        "{:indent$}{} => {} ({:.2}%)",
        "",
        prefix_stats.value.as_ref().unwrap_or(&"root".to_string()),
        prefix_stats.count,
        prefix_stats.count as f32 / parent_count as f32 * 100.,
        indent = prefix_stats.depth * 2,
    );

    let mut subkeys: Vec<&PrefixStats> = prefix_stats.subkeys.values().collect();

    if subkeys.is_empty() {
        return;
    }

    subkeys.sort_by_key(|k| k.count);
    subkeys.reverse();

    let mut other = prefix_stats.count;

    for stats in subkeys.iter() {
        other -= stats.count;
        print_stats(stats, prefix_stats.count);
    }

    let other_percentage = other as f32 / prefix_stats.count as f32 * 100.;

    if other_percentage < 1. {
        return;
    }

    println!(
        "{:indent$}{} => {} ({:.2}%)",
        "",
        "other",
        other,
        other_percentage,
        indent = (prefix_stats.depth + 1) * 2,
    );
}
