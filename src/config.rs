use clap::builder::{Styles, styling};
use clap::{Parser, ValueEnum};

use color_eyre::Result;
use color_eyre::eyre::Context as _;
use regex::Regex;

use crate::database::Database;

const CLAP_STYLING: Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());

pub struct Config {
    pub databases: Vec<Database>,
    pub all_keys_count: usize,
    pub separators: String,
    pub depth: usize,
    pub min_count_percentage: f32,
    pub progress: bool,
    pub full_keys: bool,
    pub output_format: OutputFormat,
    pub sort_order: SortOrder,
    pub scan_size: usize,
    pub memory_usage_samples: usize,
}

/// Analyzes keys in Redis to produce breakdown of the most frequent prefixes.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = CLAP_STYLING)]
struct Args {
    /// Maximum number of hosts scanned at the same time. [default: number of logical CPUs]
    #[arg(short, long, default_value_t = num_cpus::get())]
    concurrency: usize,
    /// Maximum key depth to examine.
    #[arg(short, long, default_value_t = 999)]
    depth: usize,
    /// Output format. (default: plain)
    #[arg(short, long, default_value = "plain")]
    format: OutputFormat,
    /// Shows full keys in result instead of just suffixes.
    #[arg(long)]
    full_keys: bool,
    /// Number of samples used with memory usage redis command (this is only relevant for nested values, to sample the all of the nested values, use 0)
    #[arg(long, default_value_t = 5)]
    memory_usage_samples: usize,
    /// Minimum prefix frequency in percentages for prefix to be included in the result.
    #[arg(long, default_value_t = 1.0)]
    min_count_percentage: f32,
    /// Sort order.
    #[arg(short, long, default_value = "memory_usage")]
    order: SortOrder,
    /// Shows progress
    #[arg(short, long)]
    progress: bool,
    /// Configures how many keys are fetched at a time.
    #[arg(long, default_value_t = 100)]
    scan_size: usize,
    /// List of key separators.
    #[arg(short, long, default_value = ":/|")]
    separators: String,
    /// List of URLs to scan.
    #[arg(short, long, required = true, value_delimiter = ',')]
    urls: Vec<String>,
}

impl Config {
    pub fn new() -> Result<Self> {
        let args = Args::parse();

        let databases =
            parse_and_build_databases(&args.urls).wrap_err("failed to build databases")?;
        let all_keys_count: usize = databases
            .iter()
            .fold(0, |acc, database| acc + database.keys_count);

        rayon::ThreadPoolBuilder::new()
            .num_threads(args.concurrency)
            .build_global()
            .wrap_err("failed to build thread pool")?;

        Ok(Self {
            databases,
            all_keys_count,
            separators: args.separators.to_string(),
            depth: args.depth,
            min_count_percentage: args.min_count_percentage,
            progress: args.progress,
            full_keys: args.full_keys,
            output_format: args.format,
            sort_order: args.order,
            scan_size: args.scan_size,
            memory_usage_samples: args.memory_usage_samples,
        })
    }

    pub fn separators_regex(&self) -> Regex {
        Regex::new(&format!("[{}]+", self.separators)).unwrap()
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Plain,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "snake_case")]
pub enum SortOrder {
    KeysCount,
    MemoryUsage,
}

fn parse_and_build_databases(urls: &[String]) -> Result<Vec<Database>> {
    urls.iter()
        .map(|host| {
            let url = format!("redis://{}", host);
            let client = redis::Client::open(url.as_ref())
                .wrap_err_with(|| format!("creating client ({})", host))?;
            let mut connection = client
                .get_connection()
                .wrap_err_with(|| format!("connecting ({})", host))?;
            let keys_count: usize = redis::cmd("DBSIZE")
                .query(&mut connection)
                .wrap_err_with(|| format!("getting dbsize ({})", host))?;

            Ok(Database {
                keys_count,
                connection,
            })
        })
        .collect::<Result<Vec<Database>>>()
}
