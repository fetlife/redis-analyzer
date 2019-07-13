#[macro_use]
extern crate clap;

use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use redis;
use std::sync::{Arc, Mutex};

pub mod config;
pub mod database;
pub mod prefix;
pub mod result_formatters;

use crate::config::Config;
use crate::prefix::Prefix;

fn main() {
    let mut config = Config::new();

    let mut root_prefix = Prefix::new(None, 0, config.all_keys_count);

    gather_stats(&mut config, &mut root_prefix);

    println!("");

    gather_memory_usage_stats(&mut config, &mut root_prefix);

    sort(&mut root_prefix);

    println!("");

    result_formatters::plain::call(&config, &root_prefix);
}

fn sort(prefix: &mut Prefix) {
    if prefix.children.is_empty() {
        return;
    }

    prefix.children.sort_by_key(|c| c.memory_usage);
    prefix.children.reverse();
}

fn gather_stats(config: &mut Config, prefix_stats: &mut Prefix) {
    println!(
        "Scanning {}",
        prefix_stats.value.as_ref().unwrap_or(&"root".to_string())
    );

    let frequency: HashMapFrequency<String> = HashMapFrequency::new();
    let frequency_mutex = Arc::new(Mutex::new(frequency));
    let separator = config.separators_regex();
    let bar = ProgressBar::new(prefix_stats.keys_count as u64);

    config.databases.par_iter_mut().for_each(|database| {
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

            let mut separator_positions = separator.find_iter(&key);

            let prefix = match separator_positions.nth(prefix_stats.depth) {
                None => key,
                Some(position) => unsafe { key.get_unchecked(0..position.start()) }.to_string(),
            };

            frequency_mutex.lock().unwrap().increment(prefix);
        }
    });

    bar.finish();

    for (prefix_value, count) in frequency_mutex.lock().unwrap().iter() {
        let mut child = Prefix::new(Some(prefix_value), prefix_stats.depth + 1, *count);

        let child_absolute_frequency = *count as f32 / config.all_keys_count as f32 * 100.;

        if prefix_stats.depth < config.max_depth
            && child_absolute_frequency > config.min_prefix_frequency
        {
            gather_stats(config, &mut child);
            prefix_stats.children.push(child);
        } else if prefix_stats.depth == 0 && *count > 100 {
            prefix_stats.children.push(child);
        }
    }
}

fn gather_memory_usage_stats(config: &mut Config, prefix_stats: &mut Prefix) {
    let mut cursor: u64 = 0;
    let mut iterations = 0;
    let scan_size = 100;
    let scan_size_arg = format!("{}", scan_size);

    let bar = ProgressBar::new(prefix_stats.keys_count as u64);

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

fn record_memory_usage(prefix: &mut Prefix, key: &str, memory_usage: usize) {
    if prefix
        .value
        .as_ref()
        .map_or(true, |prefix| key.starts_with(prefix))
    {
        prefix.memory_usage += memory_usage;

        for child in prefix.children.iter_mut() {
            record_memory_usage(child, key, memory_usage);
        }
    }
}
