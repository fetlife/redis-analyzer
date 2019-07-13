#[macro_use]
extern crate clap;

use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use redis;
use std::sync::{Arc, Mutex};

pub mod config;
pub mod database;
pub mod prefix;

use crate::config::Config;
use crate::prefix::Prefix;

fn main() {
    let mut config = Config::new();

    let mut root_prefix = Prefix::new(None, 0, config.all_keys_count);

    gather_stats(&mut config, &mut root_prefix);

    println!("");

    gather_memory_usage_stats(&mut config, &mut root_prefix);

    println!("");

    print_stats(&config, &root_prefix, &root_prefix);
}

pub fn gather_stats(config: &mut Config, prefix_stats: &mut Prefix) {
    println!(
        "Scanning {}",
        prefix_stats.value.as_ref().unwrap_or(&"root".to_string())
    );

    let frequency: HashMapFrequency<String> = HashMapFrequency::new();
    let frequency_mutex = Arc::new(Mutex::new(frequency));
    let delimiter = config.separators_regex();
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

            let mut delimiter_positions = delimiter.find_iter(&key);

            let prefix = match delimiter_positions.nth(prefix_stats.depth) {
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
            prefix_stats
                .children
                .insert(prefix_value.to_string(), child);
        } else if prefix_stats.depth == 0 && *count > 100 {
            prefix_stats
                .children
                .insert(prefix_value.to_string(), child);
        }
    }
}

pub fn gather_memory_usage_stats(config: &mut Config, prefix_stats: &mut Prefix) {
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

pub fn record_memory_usage(prefix_stats: &mut Prefix, key: &str, memory_usage: usize) {
    if prefix_stats
        .value
        .as_ref()
        .map_or(true, |prefix| key.starts_with(prefix))
    {
        prefix_stats.memory_usage += memory_usage;

        for (_, sub_prefix_stats) in prefix_stats.children.iter_mut() {
            record_memory_usage(sub_prefix_stats, key, memory_usage);
        }
    }
}

pub fn print_stats(config: &Config, prefix: &Prefix, parent_prefix: &Prefix) {
    println!(
        "{:indent$}{} => count: {} ({:.2}%), size: {} ({:.2}%)",
        "",
        prefix.value.as_ref().unwrap_or(&"root".to_string()),
        prefix.keys_count,
        prefix.keys_count as f32 / parent_prefix.keys_count as f32 * 100.,
        HumanBytes(prefix.memory_usage as u64),
        prefix.memory_usage as f32 / parent_prefix.memory_usage as f32 * 100.,
        indent = prefix.depth * 2,
    );

    if prefix.children.is_empty() {
        return;
    }

    let mut children: Vec<&Prefix> = prefix.children.values().collect();

    children.sort_by_key(|k| k.memory_usage);
    children.reverse();

    let mut other_keys_count = prefix.keys_count;
    let mut other_memory_usage = prefix.memory_usage;

    for child_prefix in children.iter() {
        other_keys_count -= child_prefix.keys_count;
        other_memory_usage -= child_prefix.memory_usage;
        print_stats(config, child_prefix, prefix);
    }

    let other_keys_count_percentage = other_keys_count as f32 / prefix.keys_count as f32 * 100.;

    if other_keys_count_percentage < config.min_prefix_frequency {
        return;
    }

    println!(
        "{:indent$}{} => count: {} ({:.2}%), size: {} ({:.2}%)",
        "",
        "other",
        other_keys_count,
        other_keys_count_percentage,
        HumanBytes(other_memory_usage as u64),
        other_memory_usage as f32 / prefix.memory_usage as f32 * 100.,
        indent = (prefix.depth + 1) * 2,
    );
}
