#![feature(test)]

mod ids;
mod opml;
mod rss;
mod timer;
mod utilities;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use clap::Parser;
use log::{error, info};

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
    // I have a mutable reference known_feeds. I have function periodic_tasks that receives two closures that it never calls at the same time.
    // However each of the closures needs the mutable reference known_feeds.
    // Refcell allows to check ownership at runtime instead of compile time.
    // An alternative would be to mass the known_feeds map to periodic_task which in turn
    // passes it to each closure, but this would create a dependency on the feeds
    // datatypes in generic function periodic_task
    //let mut known_feeds: rss::ExistingItemsMap = HashMap::new();
    let known_feeds: RefCell<rss::ExistingItemsMap> = RefCell::new(HashMap::new());
    let mut feed_map: HashMap<String, rss::Feed> = HashMap::new();

    timer::periodic_task(
        || {
            let feeds =
                utilities::check_and_init_feeds(&cli.so, &cli.ff, &cli.up, &cli.to).unwrap();
            for (url, filename) in &feeds {
                let current_feed = feed_map.entry(url.clone()).or_insert_with(|| {
                    // create a valid filename in target directory cli.td
                    let fully_qualified_filename = Path::new(&cli.td).join(filename);
                    // if the target directory or filename is not valid we want to panic!
                    rss::Feed::new(url, fully_qualified_filename.to_str().unwrap())
                });
                let read_result = current_feed.read();
                if let Ok(updated) = read_result {
                    if updated {
                        let mut known_feeds = known_feeds.borrow_mut();
                        let dedup_result = current_feed.remove_duplicates(&mut known_feeds);
                        if dedup_result.is_ok() {
                            let write_result = current_feed.write();
                            if write_result.is_ok() {
                                info!("Updated RSS feed {} in file {}", url, filename);
                            } else {
                                error!("Could not write updated feed {} to file {}", url, filename);
                            }
                        } else {
                            error!(
                                "Error de-duplicating feed {}: {}",
                                url,
                                dedup_result.unwrap_err()
                            );
                        }
                    } else {
                        info!("RSS feed not updated since last iteration: {}", url);
                    }
                } else {
                    error!("Error reading feed {}: {}", url, read_result.unwrap_err());
                }
            }
        },
        || {
            // at midnight we want to clear the known feeds, to reduce memory usage
            known_feeds.borrow_mut().clear();
        },
        cli.wt,
        cli.it,
    );
}
