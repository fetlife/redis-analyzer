use clap::{App, ArgMatches};
use regex::Regex;
use std::process;

use crate::database::Database;

pub struct Config {
    pub databases: Vec<Database>,
    pub all_keys_count: usize,
    pub separators: String,
    pub depth: usize,
    pub concurrency: usize,
    pub min_count_percentage: f32,
    pub progress: bool,
    pub full_keys: bool,
    pub output_format: OutputFormat,
    pub sort_order: SortOrder,
    pub scan_size: usize,
}

impl Config {
    pub fn new() -> Self {
        let yaml = load_yaml!("cli.yml");
        let arg_matches = App::from_yaml(yaml).version(crate_version!()).get_matches();

        let separators = arg_matches.value_of("separators").unwrap_or(":/|");
        let depth = arg_matches
            .value_of("depth")
            .map_or(999, |s| s.parse().expect("depth needs to be a number"));
        let min_count_percentage = arg_matches
            .value_of("min_count_percentage")
            .unwrap()
            .parse()
            .expect("min-prefix-frequency needs to be a number");

        let databases = parse_and_build_databases(&arg_matches);
        let all_keys_count: usize = databases
            .iter()
            .fold(0, |acc, database| acc + database.keys_count);

        Self {
            databases,
            all_keys_count,
            separators: separators.to_string(),
            depth,
            concurrency: parse_and_configure_concurrency(&arg_matches),
            min_count_percentage,
            progress: arg_matches.is_present("progress"),
            full_keys: arg_matches.is_present("full_keys"),
            output_format: parse_output_format(&arg_matches),
            sort_order: parse_sort_order(&arg_matches),
            scan_size: parse_usize(&arg_matches, "scan_size"),
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
    match arg_matches.value_of("order").unwrap_or("memory_usage") {
        "count" => SortOrder::KeysCount,
        "keys_count" => SortOrder::KeysCount,
        "size" => SortOrder::MemoryUsage,
        "memory" => SortOrder::MemoryUsage,
        "memory_usage" => SortOrder::MemoryUsage,
        order => {
            eprintln!("Invalid sort order: {}", order);
            process::exit(1);
        }
    }
}

fn parse_and_configure_concurrency(arg_matches: &ArgMatches) -> usize {
    let concurrency = arg_matches
        .value_of("concurrency")
        .map_or(num_cpus::get(), |s| {
            s.parse().expect("concurrency needs to be a number")
        });

    rayon::ThreadPoolBuilder::new()
        .num_threads(concurrency)
        .build_global()
        .unwrap();

    concurrency
}

fn parse_and_build_databases(arg_matches: &ArgMatches) -> Vec<Database> {
    let urls: Vec<&str> = arg_matches.value_of("urls").unwrap().split(",").collect();

    urls.iter()
        .map(|host| {
            let url = format!("redis://{}", host);
            let client =
                redis::Client::open(url.as_ref()).expect(&format!("creating client ({})", host));
            let connection = client
                .get_connection()
                .expect(&format!("connecting ({})", host));
            let keys_count: usize = redis::cmd("DBSIZE")
                .query(&connection)
                .expect(&format!("getting dbsize ({})", host));

            Database {
                host: host.to_string(),
                url,
                keys_count,
                connection,
            }
        })
        .collect()
}

fn parse_usize(arg_matches: &ArgMatches, key: &str) -> usize {
    arg_matches
        .value_of(key)
        .unwrap()
        .parse()
        .expect(&format!("{} needs to be a number", key.replace("_", "-")))
}
