use indicatif::HumanBytes;
use regex::Regex;
use std::cmp::max;

use crate::analyzer::Result;
use crate::config::Config;
use crate::key_prefix::KeyPrefix;

struct FormattingOptions {
    key_column_width: usize,
    count_column_width: usize,
    full_keys: bool,
    separators_regex: Regex,
}

pub fn call(config: &Config, result: &Result) {
    let mut options = FormattingOptions {
        key_column_width: 0,
        count_column_width: 0,
        full_keys: config.full_keys,
        separators_regex: config.separators_regex(),
    };

    let key_column_width = calculate_key_column_width(&options, &result.root_prefix);
    let count_column_width = calculate_count_column_width(&result.root_prefix);

    options.key_column_width = key_column_width;
    options.count_column_width = count_column_width;

    println!("Took {:.2?}", result.took);
    println!(
        "{:indent$}Keys Count{:indenx$}Memory Usage",
        "",
        "",
        indent = options.key_column_width,
        indenx = options.count_column_width - 9,
    );

    print_tree(
        &options,
        &result.root_prefix,
        &result.root_prefix,
        "".to_string(),
        true,
        false,
        key_column_width,
    );
}

fn print_tree(
    options: &FormattingOptions,
    node: &KeyPrefix,
    parent_node: &KeyPrefix,
    prefix: String,
    root: bool,
    last: bool,
    key_column_width: usize,
) {
    let prefix_current = if last { "└─ " } else { "├─ " };

    let display_value = display_key(options, node);

    let (leaf, info) = if root {
        let leaf = format!("{}{} ", "ALL", display_value);
        let info = info_string(options, node, parent_node, "");
        (leaf, info)
    } else {
        let leaf_prefix = format!("{}{}", prefix, prefix_current);
        let leaf = format!("{}{} ", leaf_prefix, display_value);
        let info = info_string(options, node, parent_node, &leaf_prefix);
        (leaf, info)
    };

    println!(
        "{leaf:-<width$}{info}",
        leaf = leaf,
        width = key_column_width,
        info = info,
    );

    let prefix_child = if root {
        ""
    } else if last {
        "   "
    } else {
        "│  "
    };
    let prefix = prefix + prefix_child;

    if !node.children.is_empty() {
        let last_child = node.children.len() - 1;

        for (i, child) in node.children.iter().enumerate() {
            print_tree(
                options,
                child,
                node,
                prefix.to_string(),
                false,
                i == last_child,
                key_column_width,
            );
        }
    }
}

fn display_key(options: &FormattingOptions, prefix: &KeyPrefix) -> String {
    let key = &prefix.value;

    if options.full_keys {
        return key.to_string();
    }
    let separator_positions = options.separators_regex.find_iter(key);

    let suffix = match separator_positions.last() {
        None => key,
        Some(position) => unsafe { key.get_unchecked(position.end()..key.len()) },
    };
    suffix.to_string()
}

fn info_string(
    options: &FormattingOptions,
    prefix: &KeyPrefix,
    parent_prefix: &KeyPrefix,
    leaf_prefix: &str,
) -> String {
    let mut leaf_prefix = leaf_prefix.replace(" ", "-");
    if !leaf_prefix.is_empty() {
        leaf_prefix.pop();
        leaf_prefix.push(' ');
    }
    let keys_count = display_count(prefix, parent_prefix);
    let memory_usage = format!(
        "{memory_usage} ({percentage:.2}%)",
        memory_usage = HumanBytes(prefix.memory_usage as u64),
        percentage = prefix.memory_usage as f32 / parent_prefix.memory_usage as f32 * 100.,
    );
    let leaf_prefix_with_leaf_prefix = format!(
        "{leaf_prefix}{keys_count}",
        keys_count = keys_count,
        leaf_prefix = leaf_prefix,
    );
    format!(
        "{leaf_prefix_with_leaf_prefix:-<width_left$}{leaf_prefix}{memory_usage}",
        leaf_prefix_with_leaf_prefix = leaf_prefix_with_leaf_prefix,
        leaf_prefix = leaf_prefix,
        memory_usage = memory_usage,
        width_left = options.count_column_width,
    )
}

fn display_count(prefix: &KeyPrefix, parent_prefix: &KeyPrefix) -> String {
    format!(
        "{count} ({percentage:.2}%) ",
        count = prefix.keys_count,
        percentage = prefix.keys_count as f32 / parent_prefix.keys_count as f32 * 100.,
    )
}

fn calculate_key_column_width(options: &FormattingOptions, root_prefix: &KeyPrefix) -> usize {
    let padding = 5;
    biggest_key_length(options, root_prefix) + padding
}

fn biggest_key_length(options: &FormattingOptions, prefix: &KeyPrefix) -> usize {
    let display_value = display_key(options, prefix);
    let length = display_value.len() + prefix.depth * 3;

    prefix.children.iter().fold(length, |acc, child| {
        max(acc, biggest_key_length(options, child))
    })
}

fn calculate_count_column_width(root_prefix: &KeyPrefix) -> usize {
    let padding = 4;
    biggest_count_length(root_prefix, root_prefix) + padding
}

fn biggest_count_length(
    prefix: &KeyPrefix,
    parent_prefix: &KeyPrefix,
) -> usize {
    let display_value = display_count(prefix, parent_prefix);
    let length = display_value.len() + prefix.depth * 3;

    prefix.children.iter().fold(length, |acc, child| {
        max(acc, biggest_count_length(child, prefix))
    })
}
