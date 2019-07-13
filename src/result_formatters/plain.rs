use indicatif::{HumanBytes};

use crate::config::Config;
use crate::prefix::Prefix;

pub fn call(config: &Config, root_prefix: &Prefix) {
    print_stats(&config, &root_prefix, &root_prefix);
}

fn print_stats(config: &Config, prefix: &Prefix, parent_prefix: &Prefix) {
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
