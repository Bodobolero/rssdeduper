use super::ids;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

use log::{debug, info};
use xmltree::{Element, EmitterConfig};

/// map from tuple (host, id) to tuple (channel link, item element)
pub type ExistingItemsMap = HashMap<(String, String), (String, Element)>;

pub struct Feed {
    url: String,
    filename: String,
    content: String,
    last_build_date: String,
}

fn traverse_and_modify(
    element: &mut Element,
    existing_items: &mut ExistingItemsMap,
    channel: &mut String,
) -> Result<(), String> {
    // find our own channel name and save it
    if element.name == "channel" {
        if let Some(link) = element.get_child("link") {
            *channel = link.get_text().ok_or("Channel link is empty")?.to_string();
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
                existing_items.insert(id, (channel.clone(), element.clone()));
            }
        }
    }

    element.children.retain(|child| {
        if let Some(child_element) = child.as_element() {
            if child_element.name == "item" {
                if let Some(link) = child_element.get_child("link") {
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
            traverse_and_modify(child_element, existing_items, channel)?;
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
    ) -> Result<(), String> {
        let mut rssroot = Element::parse(self.content.as_bytes())
            .map_err(|e| format!("RSS feed {} XML parse error: {}", self.url, e))?;
        let mut channel = String::new();
        traverse_and_modify(&mut rssroot, existing_items, &mut channel)?;

        let config = EmitterConfig::new()
            .indent_string("    ")
            .line_separator("\n")
            .perform_indent(true)
            .normalize_empty_elements(false);
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

        assert!(feed1.remove_duplicates(&mut existing_items).is_ok());
        assert!(feed2.remove_duplicates(&mut existing_items).is_ok());
        assert!(feed1.write().is_ok());
        assert!(feed2.write().is_ok());
        assert_eq!(4, feed1.content.matches("<item>").count());
        assert_eq!(1, feed2.content.matches("<item>").count());
        assert_eq!(0, feed1.content.matches("chifa2").count());
        assert!(feed2.content != FEED2);

        let _ = fs::remove_file(&feed1.filename);
        let _ = fs::remove_file(&feed2.filename);
    }
}
