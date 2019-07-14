use clap::{App, ArgMatches};
use regex::Regex;
use std::process;

use crate::database::Database;

pub struct Config {
    pub databases: Vec<Database>,
    pub all_keys_count: usize,
    pub separators: String,
    pub max_depth: usize,
    pub max_parallelism: usize,
    pub min_prefix_frequency: f32,
    pub progress: bool,
    pub show_full_keys: bool,
    pub output_format: OutputFormat,
    pub sort_order: SortOrder,
}

impl Config {
    pub fn new() -> Self {
        let yaml = load_yaml!("cli.yml");
        let arg_matches = App::from_yaml(yaml).version(crate_version!()).get_matches();

        let separators = arg_matches.value_of("separators").unwrap_or(":/|");
        let max_depth = arg_matches
            .value_of("max_depth")
            .map_or(999, |s| s.parse().expect("max-depth needs to be a number"));
        let min_prefix_frequency = arg_matches
            .value_of("min_prefix_frequency")
            .map_or(1., |s| {
                s.parse()
                    .expect("min-prefix-frequency needs to be a number")
            });

        let databases = parse_and_build_databases(&arg_matches);
        let all_keys_count: usize = databases
            .iter()
            .fold(0, |acc, database| acc + database.keys_count);

        Self {
            databases,
            all_keys_count,
            separators: separators.to_string(),
            max_depth,
            max_parallelism: parse_and_configure_max_parallelism(&arg_matches),
            min_prefix_frequency,
            progress: arg_matches.is_present("progress"),
            show_full_keys: arg_matches.is_present("show_full_keys"),
            output_format: parse_output_format(&arg_matches),
            sort_order: parse_sort_order(&arg_matches),
        }
    }
    pub fn separators_regex(&self) -> Regex {
        Regex::new(&format!("[{}]+", self.separators)).unwrap()
    }
}

#[derive(Debug)]
pub enum OutputFormat {
    Plain,
    Json,
}

#[derive(Debug)]
pub enum SortOrder {
    KeysCount,
    MemoryUsage,
}

fn parse_output_format(arg_matches: &ArgMatches) -> OutputFormat {
    match arg_matches.value_of("format").unwrap_or("plain") {
        "plain" => OutputFormat::Plain,
        "json" => OutputFormat::Json,
        format => {
            eprintln!("Invalid format: {}", format);
            process::exit(1);
        }
    }
}

fn parse_sort_order(arg_matches: &ArgMatches) -> SortOrder {
    match arg_matches.value_of("sort").unwrap_or("memory_usage") {
        "count" => SortOrder::KeysCount,
        "keys_count" => SortOrder::KeysCount,
        "size" => SortOrder::MemoryUsage,
        "memory" => SortOrder::MemoryUsage,
        "memory_usage" => SortOrder::MemoryUsage,
        sort_order => {
            eprintln!("Invalid sort order: {}", sort_order);
            process::exit(1);
        }
    }
}

fn parse_and_configure_max_parallelism(arg_matches: &ArgMatches) -> usize {
    let max_parallelism = arg_matches
        .value_of("max_parallelism")
        .map_or(num_cpus::get(), |s| {
            s.parse().expect("max-parallelism needs to be a number")
        });

    rayon::ThreadPoolBuilder::new()
        .num_threads(max_parallelism)
        .build_global()
        .unwrap();

    max_parallelism
}

fn parse_and_build_databases(arg_matches: &ArgMatches) -> Vec<Database> {
    let urls: Vec<&str> = arg_matches.value_of("urls").unwrap().split(",").collect();

    urls.iter()
        .map(|host| {
            let url = format!("redis://{}", host);
            let client = redis::Client::open(url.as_ref()).expect("connect to redis");
            let connection = client.get_connection().expect("getting connection");
            let keys_count: usize = redis::cmd("DBSIZE")
                .query(&connection)
                .expect("getting dbsize");

            Database {
                host: host.to_string(),
                url,
                keys_count,
                connection,
            }
        })
        .collect()
}
