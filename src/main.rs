#![feature(test)]

mod ids;
mod opml;
mod timer;
mod utilities;

use std::collections::HashMap;

use clap::Parser;
use log::info;
use xmltree::Element;

/// This struct defines the command line interface for the application
#[derive(Parser, Debug)]
#[clap(
    author = "github.com/bodobolero/rssdeduper/",
    version,
    about = "Deduplicate RSS feeds",
    long_about = r#"See https://github.com/Bodobolero/rssdeduper/README.md for more information.
To see logging information invoke with
RUST_LOG=info"#
)]
struct Cli {
    /// Sets the source OPML filename
    #[clap(long, value_name = "FILE", default_value = "./feedly-source.opml")]
    so: String,

    /// Sets the target OPML filename
    #[clap(long, value_name = "FILE", default_value = "./feedly-target.opml")]
    to: String,

    /// Sets the target feed file
    #[clap(long, value_name = "FILE", default_value = "./feeds.json")]
    ff: String,

    /// Sets the target directory for rss feeds
    #[clap(long, value_name = "DIRECTORY", default_value = "/var/www/html/rss/")]
    td: String,

    /// Sets the url prefix to be used in the target OPML file
    #[clap(
        long,
        value_name = "URL",
        default_value = "https://www.bodobolero.com/rss/"
    )]
    up: String,

    /// Sets the wait time in seconds between iterations
    #[clap(long, value_name = "SECONDS", default_value = "60")]
    wt: u64,

    /// Sets the maximum number of iterations, default 0 means unlimited
    #[clap(long, value_name = "ITERATIONS", default_value = "0")]
    it: u64,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    info!("Source OPML filename: {}", cli.so);
    info!("Target OPML filename: {}", cli.to);
    info!("URL prefix: {}", cli.up);
    info!("Target feed file: {}", cli.ff);
    info!("Target directory for rss feeds: {}", cli.td);
    info!("Iteration wait time: {} seconds", cli.wt);
    info!("Maximum number of iterations: {}", cli.it);

    let mut known_feeds: HashMap<String, (String, Element)> = HashMap::new();

    timer::periodic_task(
        || {
            let _feeds =
                utilities::check_and_init_feeds(&cli.so, &cli.ff, &cli.up, &cli.to, &cli.td)
                    .unwrap();
        },
        || {
            // at midnight we want to clear the known feeds, to reduce memory usage
            known_feeds.clear();
        },
        cli.wt,
        cli.it,
    );
}
