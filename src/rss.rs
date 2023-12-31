use super::ids;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

use chrono::{DateTime, Duration, Local, Utc};
use log::{debug, info};
use std::time::SystemTime;
use xmltree::{Element, EmitterConfig};

/// map from tuple (host, id) to tuple (channel link, item element, timestamp added to map)
pub type ExistingItemsMap = HashMap<(String, String), (String, Element, SystemTime)>;

pub struct Feed {
    url: String,
    filename: String,
    content: String,
    last_build_date: String,
}

// max_age in hours
fn check_pub_date_not_older_than(pub_date: &str, max_age: u64, nowutc: &DateTime<Utc>) -> bool {
    if max_age == 0 {
        // 0 == unlimited
        return true;
    }
    if let Ok(pub_date_utc) =
        DateTime::parse_from_rfc2822(pub_date).map(|dt| dt.with_timezone(&Utc))
    {
        if nowutc.signed_duration_since(pub_date_utc) <= Duration::hours(max_age as i64) {
            return true;
        }
    }
    false
}

fn traverse_and_modify(
    element: &mut Element,
    existing_items: &mut ExistingItemsMap,
    channel: &mut String,
    max_age: u64, // in hours
) -> Result<(), String> {
    let now = SystemTime::now();
    let nowutc = Local::now().with_timezone(&Utc);
    // find our own channel name and save it
    if element.name == "channel" {
        if let Some(link) = element.get_child("link") {
            if let Some(text) = link.get_text() {
                *channel = text.to_string();
            } else {
                debug!("Channel link text is empty, trying href");
                *channel = link
                    .attributes
                    .get("href")
                    .ok_or("Channel link href is empty, too")?
                    .to_string();
            }
        } else {
            return Err("Channel link is missing".to_string());
        }
    }
    if element.name == "item" {
        if let Some(link) = element.get_child("link") {
            let id = ids::extract_unique_id_and_host_from_url_string(
                &link.get_text().unwrap_or_default(),
            )
            .unwrap_or_default();
            if let Some(existing) = existing_items.get(&id) {
                if existing.0 == *channel {
                    info!(
                        "Replacing duplicate item {} in same channel {}",
                        link.get_text().unwrap_or_default(),
                        channel
                    );
                    element.children.clear();
                    for node in &existing.1.children {
                        element.children.push(node.clone());
                    }
                }
            } else {
                // if not yet existing insert it
                existing_items.insert(id, (channel.clone(), element.clone(), now));
            }
        }
    }

    element.children.retain(|child| {
        if let Some(child_element) = child.as_element() {
            if child_element.name == "item" {
                
                if let Some(link) = child_element.get_child("link") {
                    // remove old items first
                    if let Some(pubdate) = child_element.get_child("pubDate"){
                        if !check_pub_date_not_older_than(&pubdate.get_text().unwrap_or_default(), max_age, &nowutc){
                            info!("Removing old item {} with pubDate {}", link.get_text().unwrap_or_default(), pubdate.get_text().unwrap_or_default());
                            return false;
                        }
                    }
                    let id = ids::extract_unique_id_and_host_from_url_string(
                        &link.get_text().unwrap_or_default(),
                    ).unwrap_or_default();
                    if let Some(existing) = existing_items.get(&id) {
                        if existing.0 == *channel {
                            debug!(
                                "Keping duplicate item {} from same channel {} to be replaced later",
                                link.get_text().unwrap_or_default(),
                                channel
                            );
                            return true;
                        } else {
                            info!(
                                "Removing duplicate item {}, previous channel {}, current channel {}",
                                link.get_text().unwrap_or_default(),
                                existing.0,
                                channel
                            );
                            return false;
                        }
                    } else {
                        debug!(
                            "Keeping new item {} from channel {}",
                            link.get_text().unwrap_or_default(),
                            channel
                        );
                        return true;
                    }
                }
            }
        }
        true
    });

    // Recursively modify child elements
    for child in element.children.iter_mut() {
        if let Some(child_element) = child.as_mut_element() {
            traverse_and_modify(child_element, existing_items, channel, max_age)?;
        }
    }
    Ok(())
}

impl Feed {
    pub fn new(url: &str, filename: &str) -> Self {
        Self {
            url: url.to_string(),
            filename: filename.to_string(),
            content: String::new(),
            last_build_date: String::new(),
        }
    }

    // read the content of the stream into an internal String and return if the feed has been updated
    // from the last time it was read
    pub fn read(&mut self) -> Result<bool, String> {
        let response = reqwest::blocking::get(&self.url)
            .map_err(|e| format!("Feed {} cannot be read: {}", self.url, e))?;
        self.content = response
            .text()
            .map_err(|e| format!("Feed {} cannot be read: {}", self.url, e))?;
        // use simple parsing for lastbuildDate to avoid full xml parsing if content hasn't changed
        for line in self.content.lines() {
            if line.trim_start().starts_with("<lastBuildDate>") {
                let modified = self.last_build_date != line.trim();
                self.last_build_date = line.trim().to_string();
                info!(
                    "Feed has {}been updated: {}",
                    if modified { "" } else { "not " },
                    self.url,
                );
                return Ok(modified);
            }
        }
        // if we don't have lastBuildDate we assume it has been updated and refresh in each iteration
        Ok(true)
    }

    /*
    - for each item in the feed
    - create the ID for the item
    - if the ID is not in the HashMap keys add it to the HashMap and publish the item to the feed
    - if the ID is in the HashMap keys and the feed is the same feed as the one in the HashMap value publish the original item (not the new one) to the feed
    - if the ID is in the HashMap keys and the feed is different from the one in the HashMap value do not publish the item
     existing_items: - HashMap<ID, (channellink, content)>  a map from the item ID (generated from the item link) to a tuple containing the channel link URL and the item XML elements

     */
    pub fn remove_duplicates(
        &mut self,
        existing_items: &mut ExistingItemsMap,
        max_age: u64
    ) -> Result<(), String> {
        let mut rssroot = Element::parse(self.content.as_bytes())
            .map_err(|e| format!("RSS feed {} XML parse error: {}", self.url, e))?;
        let mut channel = String::new();
        traverse_and_modify(&mut rssroot, existing_items, &mut channel, max_age)?;

        let config = EmitterConfig::new()
            .indent_string("    ")
            .line_separator("\n")
            .perform_indent(true)
            .normalize_empty_elements(true);
        let mut new_content = Vec::with_capacity(self.content.len());
        rssroot
            .write_with_config(&mut new_content, config)
            .map_err(|e| format!("RSS feed {} XML write error: {}", self.url, e))?;
        self.content = String::from_utf8(new_content).unwrap();
        Ok(())
    }

    // write the content of the feed to its file
    pub fn write(&self) -> Result<(), String> {
        let tmp_filename = format!("{}.tmp", &self.filename);
        let mut file = fs::File::create(&tmp_filename)
            .map_err(|e| format!("Temporary file {} cannot be created: {}", tmp_filename, e))?;
        file.write_all(self.content.as_bytes())
            .map_err(|e| format!("Temporary file {} cannot be written: {}", tmp_filename, e))?;
        fs::rename(&tmp_filename, &self.filename)
            .map_err(|e| format!("File {} cannot be renamed: {}", tmp_filename, e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::utilities::setup_test_logger;
    use super::*;

    #[test]
    fn test_rss_read() {
        setup_test_logger();
        let mut feed = Feed::new(
            "http://www.faz.net/aktuell/politik/ausland/?rssview=1",
            "testdata/testfazausland.rss",
        );
        let result = feed.read();
        assert!(result.is_ok());
        assert!(result.unwrap()); // updated == true
        assert!(!feed.content.is_empty());
        let result = feed.read();
        assert!(result.is_ok());
        assert!(!result.unwrap()); // updated == false
        assert!(!feed.content.is_empty());
    }

    #[test]
    fn test_rss_write() {
        setup_test_logger();
        let mut feed = Feed::new(
            "http://www.faz.net/aktuell/politik/ausland/?rssview=1",
            "testdata/testfazausland.rss",
        );
        // feed.content = String::from("Test");
        let result = feed.read();
        assert!(result.is_ok());
        let result = feed.write();
        assert!(result.is_ok());
        assert!(Path::new(&feed.filename).exists());
        let _ = fs::remove_file(&feed.filename);
    }

    #[test]
    fn test_rss_remove_duplicates() {
        const FEED1: &str = include_str!("../testdata/channel1.rss");
        const FEED2: &str = include_str!("../testdata/channel2.rss");
        setup_test_logger();
        let mut feed1 = Feed::new(
            "https://www.stuttgarter-zeitung.de/news",
            "testdata/channel1_dedup.rss",
        );
        feed1.content = FEED1.to_string();
        let mut feed2 = Feed::new(
            "https://www.stuttgarter-zeitung.de/schlagzeilen",
            "testdata/channel2_dedup.rss",
        );
        feed2.content = FEED2.to_string();
        let mut existing_items: ExistingItemsMap = HashMap::new();

        assert!(feed1.remove_duplicates(&mut existing_items, 0).is_ok());
        assert!(feed2.remove_duplicates(&mut existing_items, 0).is_ok());
        assert!(feed1.write().is_ok());
        assert!(feed2.write().is_ok());
        assert_eq!(4, feed1.content.matches("<item>").count());
        assert_eq!(1, feed2.content.matches("<item>").count());
        assert_eq!(0, feed1.content.matches("chifa2").count());
        assert!(feed2.content != FEED2);

        let _ = fs::remove_file(&feed1.filename);
        let _ = fs::remove_file(&feed2.filename);
    }

    #[test]
    fn test_rss_with_atom_link() {
        const FEED1: &str = include_str!("../testdata/feedwithatomlink.rss");
        setup_test_logger();
        let mut feed1 = Feed::new(
            "http://arduino-praxis.ch/feed/",
            "testdata/atomlink_dedup.rss",
        );
        feed1.content = FEED1.to_string();
        let mut existing_items: ExistingItemsMap = HashMap::new();
        let result = feed1.remove_duplicates(&mut existing_items, 0);
        // info!("Result: {:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rss_remove_duplicates_with_small_maxage() {
        const FEED1: &str = include_str!("../testdata/channel1.rss");
        setup_test_logger();
        let mut feed1 = Feed::new(
            "https://www.stuttgarter-zeitung.de/news",
            "testdata/channel1_dedup_with_age.rss",
        );
        feed1.content = FEED1.to_string();
        
        let mut existing_items: ExistingItemsMap = HashMap::new();

        assert!(feed1.remove_duplicates(&mut existing_items, 1).is_ok());
        assert!(feed1.write().is_ok());
        assert_eq!(0, feed1.content.matches("<item>").count());

        let _ = fs::remove_file(&feed1.filename);

    }

    #[test]
    fn test_rss_remove_duplicates_with_large_maxage() {
        const FEED1: &str = include_str!("../testdata/channel1.rss");
        setup_test_logger();
        let mut feed1 = Feed::new(
            "https://www.stuttgarter-zeitung.de/news",
            "testdata/channel1_dedup_with_large_age.rss",
        );
        feed1.content = FEED1.to_string();
        
        let mut existing_items: ExistingItemsMap = HashMap::new();

        assert!(feed1.remove_duplicates(&mut existing_items,std::u32::MAX as u64 ).is_ok());
        assert!(feed1.write().is_ok());
        assert_eq!(4, feed1.content.matches("<item>").count());

        let _ = fs::remove_file(&feed1.filename);

    }

}
