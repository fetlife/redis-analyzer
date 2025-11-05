# Redis Analyzer

Console tool to scan keys in Redis database in real time and aggregate count and memory usage statistics by key prefixes.

Features:

* Keys count and memory usage statistics
* Concurrent scanning of multiple Redis databases
* Low memory signature
* Fast (scans instances with dozens of millions of keys in few minutes)

## Motivation

There are already good tools out there doing similar job with even more features:

* [redis-memory-analyzer](https://github.com/gamenet/redis-memory-analyzer)
* [redis-audit](https://github.com/snmaynard/redis-audit)

But using them on big Redis databases with millions of keys is not viable because of memory requirements and time it takes to complete the full scan.

## Installation

You'll need rust and cargo. See [here](https://doc.rust-lang.org/cargo/getting-started/installation.html) for instructions on how to get them.

Then you can just run `cargo install redis-analyzer` to install it.

Alternatively, to build it yourself, clone the repository and run `cargo build --release`.

## Usage

```text
# redis-analyzer --help

Analyzes keys in Redis to produce breakdown of the most frequent prefixes.

Usage: redis-analyzer [OPTIONS] --urls <URLS>

Options:
  -c, --concurrency <CONCURRENCY>
          Maximum number of hosts scanned at the same time. [default: number of logical CPUs] [default: 10]
  -d, --depth <DEPTH>
          Maximum key depth to examine [default: 999]
  -f, --format <FORMAT>
          Output format. (default: plain) [default: plain] [possible values: plain, json]
      --full-keys
          Shows full keys in result instead of just suffixes
      --memory-usage-samples <MEMORY_USAGE_SAMPLES>
          Number of samples used with memory usage redis command (this is only relevant for nested values, to sample the all of the nested values, use 0) [default: 5]
      --min-count-percentage <MIN_COUNT_PERCENTAGE>
          Minimum prefix frequency in percentages for prefix to be included in the result [default: 1]
  -o, --order <ORDER>
          Sort order [default: memory_usage] [possible values: keys_count, memory_usage]
  -p, --progress
          Shows progress
      --scan-size <SCAN_SIZE>
          Configures how many keys are fetched at a time [default: 100]
  -s, --separators <SEPARATORS>
          List of key separators [default: :/|]
  -u, --urls <URLS>
          List of URLs to scan
  -h, --help
          Print help
  -V, --version
          Print version
```

## Examples

Example URLs:

* Basic: `redis://127.0.0.1:6379`
* Username & Password: `redis://user:password@127.0.0.1:6379`
* Password only: `redis://:password@127.0.0.1:6379`
* Specifying DB: `redis://127.0.0.1:6379/0`
* Enabling TLS: `rediss://127.0.0.1:6379`
* Enabling Insecure TLS: `rediss://127.0.0.1:6379/#insecure`
* Enabling RESP3: `redis://127.0.0.1:6379/?protocol=resp3`

Example output:

```text
$ redis-analyzer -u 127.0.0.1:6379/0,127.0.0.1:6379/2
Took 976ms 142us
                       Keys Count                     Memory Usage
ALL -------------------15155 (100.00%) --------------26.88MB (100.00%)
├─ cache --------------├─ 294 (1.94%) ---------------├─ 2.04MB (7.60%)
│  └─ Touchify --------│--└─ 239 (81.29%) -----------│--└─ 20.62KB (0.99%)
│     └─ internal -----│-----└─ 239 (100.00%) -------│-----└─ 20.62KB (100.00%)
│        └─ User ------│--------└─ 239 (100.00%) ----│--------└─ 20.62KB (100.00%)
├─ feed ---------------├─ 158 (1.04%) ---------------├─ 1.60MB (5.97%)
│  └─ feed ------------│--└─ 158 (100.00%) ----------│--└─ 1.60MB (100.00%)
│     └─ feeds --------│-----└─ 155 (98.10%) --------│-----└─ 1.60MB (99.93%)
├─ hovno --------------├─ 13808 (91.11%) ------------├─ 1.55MB (5.75%)
├─ sidekiq ------------├─ 399 (2.63%) ---------------├─ 1.27MB (4.74%)
│  └─ stat ------------│--└─ 388 (97.24%) -----------│--└─ 28.21KB (2.16%)
│     ├─ processed ----│-----├─ 194 (50.00%) --------│-----├─ 14.58KB (51.68%)
│     └─ failed -------│-----└─ 194 (50.00%) --------│-----└─ 13.63KB (48.32%)
├─ stat ---------------├─ 176 (1.16%) ---------------├─ 11.24KB (0.04%)
├─ counts -------------├─ 120 (0.79%) ---------------├─ 9.68KB (0.04%)
└─ [other] ------------└─ 200 (1.32%) ---------------└─ 20.39MB (75.86%)
```

```text
$ redis-analyzer -u 127.0.0.1:6379/0 -d 1
Took 752ms 708us
               Keys Count             Memory Usage
ALL -----------14971 (100.00%) ------7.29MB (100.00%)
├─ cache ------├─ 294 (1.96%) -------├─ 2.04MB (28.02%)
├─ feed -------├─ 158 (1.06%) -------├─ 1.60MB (22.00%)
├─ hovno ------├─ 13808 (92.23%) ----├─ 1.55MB (21.19%)
├─ sidekiq ----├─ 399 (2.67%) -------├─ 1.27MB (17.42%)
├─ counts -----├─ 120 (0.80%) -------├─ 9.68KB (0.13%)
└─ [other] ----└─ 192 (1.28%) -------└─ 840.05KB (11.25%)
```

```text
$ redis-analyzer -u 127.0.0.1:6379/0 -d 2 --order count --full-keys
Took 838ms 811us
                         Keys Count               Memory Usage
ALL ---------------------14971 (100.00%) --------7.29MB (100.00%)
├─ hovno ----------------├─ 13808 (92.23%) ------├─ 1.55MB (21.19%)
├─ sidekiq --------------├─ 399 (2.67%) ---------├─ 1.27MB (17.42%)
│  └─ sidekiq:stat ------│--└─ 388 (97.24%) -----│--└─ 28.21KB (2.17%)
├─ cache ----------------├─ 294 (1.96%) ---------├─ 2.04MB (28.02%)
│  └─ cache:Touchify ----│--└─ 239 (81.29%) -----│--└─ 20.62KB (0.99%)
├─ feed -----------------├─ 158 (1.06%) ---------├─ 1.60MB (22.00%)
│  └─ feed:feed ---------│--└─ 158 (100.00%) ----│--└─ 1.60MB (100.00%)
├─ counts ---------------├─ 120 (0.80%) ---------├─ 9.68KB (0.13%)
└─ [other] --------------└─ 192 (1.28%) ---------└─ 840.05KB (11.25%)
```
