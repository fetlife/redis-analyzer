use indicatif::HumanBytes;

use crate::config::Config;
use crate::prefix::Prefix;

pub fn call(config: &Config, root_prefix: &Prefix) {
    println!(
        "{:indent$}Key Count{:indenx$}Memory Usage",
        "",
        "",
        indent = 29,
        indenx = 31,
    );

    print_tree(
        &config,
        &root_prefix,
        &root_prefix,
        "".to_string(),
        true,
        false,
    );
}

fn print_tree(
    config: &Config,
    node: &Prefix,
    parent_node: &Prefix,
    prefix: String,
    root: bool,
    last: bool,
) {
    let prefix_current = if last { " └─ " } else { " ├─ " };
    let display_value = key_suffix(node.value.as_ref().unwrap_or(&"".to_string()), config);

    let (leaf, info) = if root {
        let leaf = format!("{}{} ", "ALL", display_value);
        let info = info_string(node, parent_node, "");
        (leaf, info)
    } else {
        let leaf_prefix = format!("{}{}", prefix, prefix_current);
        let leaf = format!("{}{} ", leaf_prefix, display_value);
        let info = info_string(node, parent_node, &leaf_prefix);
        (leaf, info)
    };

    println!(
        "{leaf:-<width$}{info}",
        leaf = leaf,
        width = 30,
        info = info,
    );

    let prefix_child = if root {
        ""
    } else if last {
        "   "
    } else {
        " │  "
    };
    let prefix = prefix + prefix_child;

    if !node.children.is_empty() {
        let last_child = node.children.len() - 1;

        for (i, child) in node.children.iter().enumerate() {
            print_tree(
                config,
                &child,
                node,
                prefix.to_string(),
                false,
                i == last_child,
            );
        }
    }
}

pub fn key_suffix(key: &str, config: &Config) -> String {
    let separator = config.separators_regex();
    let separator_positions = separator.find_iter(&key);

    let suffix = match separator_positions.last() {
        None => key,
        Some(position) => unsafe { key.get_unchecked(position.end()..key.len()) },
    };
    suffix.to_string()
}

fn info_string(prefix: &Prefix, parent_prefix: &Prefix, leaf_prefix: &str) -> String {
    let mut leaf_prefix = leaf_prefix.replace(" ", "-");
    leaf_prefix.pop();
    leaf_prefix.push(' ');
    let keys_count = format!(
        "{count} ({percentage:.2}%)",
        count = prefix.keys_count,
        percentage = prefix.keys_count as f32 / parent_prefix.keys_count as f32 * 100.,
    );
    let memory_usage = format!(
        "{memory_usage} ({percentage:.2}%) ",
        memory_usage = format!("{}", HumanBytes(prefix.memory_usage as u64)),
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
        width_left = 40,
    )
}
