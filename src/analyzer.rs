use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use redis;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::config::{Config, SortOrder};
use crate::key_prefix::KeyPrefix;

pub struct Result {
    pub root_prefix: KeyPrefix,
    pub took: Duration,
}

pub fn run(config: &mut Config) -> Result {
    let mut root_prefix = KeyPrefix::new("", 0, config.all_keys_count, 0);

    let now = SystemTime::now();

    analyze_count(config, &mut root_prefix);
    analyze_memory_usage(config, &mut root_prefix);
    reorder(&config, &mut root_prefix);
    backfill_other_keys(&config, &mut root_prefix);

    let took = now.elapsed().unwrap();

    Result { root_prefix, took }
}

fn analyze_count(config: &mut Config, prefix: &mut KeyPrefix) {
    let frequency: HashMapFrequency<String> = HashMapFrequency::new();
    let frequency_mutex = Arc::new(Mutex::new(frequency));
    let separator = config.separators_regex();
    let bar = if config.progress {
        println!("Scanning {}", prefix.value,);
        ProgressBar::new(prefix.keys_count as u64)
    } else {
        ProgressBar::hidden()
    };

    let scan_size = config.scan_size;

    config.databases.par_iter_mut().for_each(|database| {
        bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "[{elapsed_precise}] {wide_bar} {pos}/{len} ({percent}%) [ETA: {eta_precise}]",
                )
                .expect("Failed to set progress bar style"),
        );

        let mut scan_command = redis::cmd("SCAN")
            .cursor_arg(0)
            .arg("COUNT")
            .arg(scan_size)
            .clone();

        if !prefix.value.is_empty() {
            scan_command = scan_command
                .arg("MATCH")
                .arg(format!("{}*", prefix.value))
                .clone();
        }

        let iter: redis::Iter<String> = scan_command
            .clone()
            .iter(&mut database.connection)
            .expect("running scan");

        for (i, key) in iter.enumerate() {
            if i % 10_000 == 0 && i > 0 {
                bar.inc(10_000);
            }

            let mut separator_positions = separator.find_iter(&key);

            let prefix = match separator_positions.nth(prefix.depth) {
                None => key,
                Some(position) => unsafe { key.get_unchecked(0..position.start()) }.to_string(),
            };

            frequency_mutex.lock().unwrap().increment(prefix);
        }
    });

    bar.finish();

    for (prefix_value, count) in frequency_mutex.lock().unwrap().iter() {
        let mut child = KeyPrefix::new(prefix_value, prefix.depth + 1, *count, 0);

        let child_absolute_frequency = *count as f32 / config.all_keys_count as f32 * 100.;

        if prefix.depth < config.depth && child_absolute_frequency > config.min_count_percentage {
            analyze_count(config, &mut child);
            prefix.children.push(child);
        }
    }
}

fn analyze_memory_usage(config: &mut Config, prefix: &mut KeyPrefix) {
    let bar = if config.progress {
        println!();
        println!("Memory scanning");
        ProgressBar::new(prefix.keys_count as u64)
    } else {
        ProgressBar::hidden()
    };

    bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "[{elapsed_precise}] {wide_bar} {pos}/{len} ({percent}%) [ETA: {eta_precise}]",
            )
            .expect("failed to set progress bar style"),
    );

    let scan_size = config.scan_size;
    let memory_usage_samples = config.memory_usage_samples;
    let prefix_mutex = Arc::new(Mutex::new(prefix));

    config.databases.par_iter_mut().for_each(|database| {
        let mut cursor: u64 = 0;

        loop {
            let scan_command = redis::cmd("SCAN")
                .cursor_arg(cursor)
                .arg("COUNT")
                .arg(scan_size)
                .clone();

            let (new_cursor, keys): (u64, Vec<String>) =
                scan_command.query(&mut database.connection).expect("scan");

            cursor = new_cursor;

            let mut memory_usage_command = redis::pipe().clone();

            for key in keys.iter() {
                memory_usage_command = memory_usage_command
                    .cmd("MEMORY")
                    .arg("USAGE")
                    .arg(key)
                    .arg("SAMPLES")
                    .arg(memory_usage_samples)
                    .clone();
            }

            let memory_usages: Vec<usize> = memory_usage_command
                .query(&mut database.connection)
                .expect("memory usage command");

            bar.inc(keys.len() as u64);

            for (key, memory_usage) in keys.iter().zip(memory_usages.iter()) {
                record_memory_usage(prefix_mutex.lock().unwrap().deref_mut(), key, *memory_usage);
            }

            if cursor == 0 {
                break;
            }
        }
    });

    bar.finish();
}

fn record_memory_usage(prefix: &mut KeyPrefix, key: &str, memory_usage: usize) {
    if key.starts_with(&prefix.value) {
        prefix.memory_usage += memory_usage;

        for child in prefix.children.iter_mut() {
            record_memory_usage(child, key, memory_usage);
        }
    }
}

fn reorder(config: &Config, prefix: &mut KeyPrefix) {
    if prefix.children.is_empty() {
        return;
    }

    prefix.children.sort_by_key(|c| match config.sort_order {
        SortOrder::KeysCount => c.keys_count,
        SortOrder::MemoryUsage => c.memory_usage,
    });
    prefix.children.reverse();
}

fn backfill_other_keys(config: &Config, prefix: &mut KeyPrefix) {
    if prefix.children.is_empty() {
        return;
    }

    let mut other_prefix = KeyPrefix::new(
        "[other]",
        prefix.depth + 1,
        prefix.keys_count,
        prefix.memory_usage,
    );

    for child in prefix.children.iter() {
        other_prefix.keys_count -= child.keys_count;
        other_prefix.memory_usage -= child.memory_usage;
    }

    let other_absolute_frequency =
        other_prefix.keys_count as f32 / config.all_keys_count as f32 * 100.;

    if other_absolute_frequency > config.min_count_percentage {
        prefix.children.push(other_prefix);
    }
}
