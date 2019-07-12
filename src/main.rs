#[macro_use]
extern crate clap;

use clap::App;
use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
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

pub struct Database {
    pub keys_count: usize,
    pub connection: redis::Connection,
}

pub struct Config {
    pub databases: Vec<Database>,
    pub all_keys_count: usize,
    pub separators: String,
    pub max_depth: usize,
}

impl Config {
    pub fn separators_regex(&self) -> Regex {
        Regex::new(&format!("[{}]+", self.separators)).unwrap()
    }
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let urls: Vec<&str> = matches.value_of("urls").unwrap().split(",").collect();
    let separators = matches.value_of("separators").unwrap_or(":/|");
    let max_depth = matches
        .value_of("max_depth")
        .map_or(999, |s| s.parse().expect("max-depth needs to be a number"));

    let databases: Vec<Database> = urls
        .iter()
        .map(|host| {
            let client = redis::Client::open(format!("redis://{}", host).as_ref())
                .expect("connect to redis");
            let connection = client.get_connection().expect("getting connection");
            let keys_count: usize = redis::cmd("DBSIZE")
                .query(&connection)
                .expect("getting dbsize");

            Database {
                keys_count,
                connection,
            }
        })
        .collect();

    let all_keys_count: usize = databases
        .iter()
        .fold(0, |acc, database| acc + database.keys_count);

    let mut config = Config {
        databases,
        all_keys_count,
        separators: separators.to_string(),
        max_depth,
    };

    let mut top_stats = PrefixStats::new(None, 0, all_keys_count);

    gather_stats(&mut top_stats, &mut config);

    println!("");

    gather_memory_usage_stats(&mut top_stats, &mut config);

    println!("");

    print_stats(&top_stats, top_stats.memory_usage);
}

pub fn gather_stats(prefix_stats: &mut PrefixStats, config: &mut Config) {
    println!(
        "Scanning {}",
        prefix_stats.value.as_ref().unwrap_or(&"root".to_string())
    );

    let mut frequency: HashMapFrequency<String> = HashMapFrequency::new();
    let delimiter = config.separators_regex();
    let bar = ProgressBar::new(prefix_stats.count as u64);

    for database in config.databases.iter_mut() {
        bar.set_style(ProgressStyle::default_bar().template(
            "[{elapsed_precise}] {wide_bar} {pos}/{len} ({percent}%) [ETA: {eta_precise}]",
        ));

        let mut scan_command = redis::cmd("SCAN")
            .cursor_arg(0)
            .arg("COUNT")
            .arg("100")
            .clone();

        if let Some(p) = &prefix_stats.value {
            scan_command = scan_command.arg("MATCH").arg(format!("{}*", p)).clone();
        }

        let iter: redis::Iter<String> = scan_command
            .clone()
            .iter(&database.connection)
            .expect("running scan");

        for (i, key) in iter.enumerate() {
            if i % 10_000 == 0 && i > 0 {
                bar.inc(10_000);
            }

            let mut delimiter_positions = delimiter.find_iter(&key);

            let prefix = match delimiter_positions.nth(prefix_stats.depth) {
                None => key,
                Some(position) => unsafe { key.get_unchecked(0..position.start()) }.to_string(),
            };

            frequency.increment(prefix);
        }
    }

    bar.finish();

    for (prefix, count) in frequency.iter() {
        let mut subkey = PrefixStats::new(Some(prefix), prefix_stats.depth + 1, *count);

        // if key count is larger than 1% of all keys count
        if prefix_stats.depth < config.max_depth && *count > config.all_keys_count / 100 {
            gather_stats(&mut subkey, config);
            prefix_stats.subkeys.insert(prefix.to_string(), subkey);
        } else if prefix_stats.depth == 0 && *count > 100 {
            prefix_stats.subkeys.insert(prefix.to_string(), subkey);
        }
    }
}

pub fn gather_memory_usage_stats(prefix_stats: &mut PrefixStats, config: &mut Config) {
    let mut cursor: u64 = 0;
    let mut iterations = 0;
    let scan_size = 100;
    let scan_size_arg = format!("{}", scan_size);

    let bar = ProgressBar::new(prefix_stats.count as u64);

    bar.set_message("Memory scanning");
    bar.set_style(ProgressStyle::default_bar().template(
        "{msg}\n[{elapsed_precise}] {wide_bar} {pos}/{len} ({percent}%) [ETA: {eta_precise}]",
    ));

    for database in config.databases.iter_mut() {
        loop {
            if iterations % 1_000 == 0 && iterations > 0 {
                bar.inc(1_000)
            }

            let scan_command = redis::cmd("SCAN")
                .cursor_arg(cursor)
                .arg("COUNT")
                .arg(&scan_size_arg)
                .clone();

            let (new_cursor, keys): (u64, Vec<String>) =
                scan_command.query(&database.connection).expect("scan");

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
                .query(&database.connection)
                .expect("memory usage command");

            iterations += keys.len();

            for (key, memory_usage) in keys.iter().zip(memory_usages.iter()) {
                record_memory_usage(prefix_stats, key, *memory_usage);
            }

            if cursor == 0 {
                break;
            }
        }
    }

    bar.finish();
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
        HumanBytes(prefix_stats.memory_usage as u64),
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
        HumanBytes(other_memory_usage as u64),
        other_percentage,
        indent = (prefix_stats.depth + 1) * 2,
    );
}
