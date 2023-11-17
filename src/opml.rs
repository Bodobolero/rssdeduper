use super::ids::{convert_url_to_unique_filename, generate_uuid};

use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use xmltree::{Element, EmitterConfig};

pub struct OpmlDom {
    opmlroot: Element,
    filename: String,
    feeds: Vec<(String, String)>,
}

// serialize and deserialize feeds to file
#[derive(Serialize, Deserialize, Debug)]
struct Data(Vec<(String, String)>);

fn modify_text_title_and_xmlurl_and_collect_changes(
    element: &mut Element,
    new_url_prefix: String,
    collector: &mut Vec<(String, String)>,
    previous_feeds: &HashMap<String, String>,
) {
    if element.name == "outline" {
        let mut newfeed = true;
        if let Some(title) = element.attributes.get_mut("title") {
            if title.starts_with("DD_") {
                newfeed = false;
            } else {
                title.insert_str(0, "DD_");
            }
        }
        if let Some(text) = element.attributes.get_mut("text") {
            if newfeed {
                text.insert_str(0, "DD_");
            }
        }
        if let Some(xmlurl) = element.attributes.get_mut("xmlUrl") {
            if newfeed {
                let old_xmlurl = xmlurl.clone();
                let new_filename = convert_url_to_unique_filename(xmlurl, &generate_uuid());
                *xmlurl = new_url_prefix + new_filename.as_str();
                info!("Added new feed {} with url {}", &old_xmlurl, &new_filename);
                collector.push((old_xmlurl, new_filename));
            } else {
                // extract the feedfile from the xmlurl
                let feedfile = xmlurl
                    .clone()
                    .strip_prefix(new_url_prefix.as_str())
                    .unwrap_or(xmlurl)
                    .to_string();
                // lookup the sourceurl in the previous feeds
                if let Some(sourceurl) = previous_feeds.get(feedfile.as_str()) {
                    collector.push((sourceurl.clone(), feedfile.to_string()));
                } else {
                    error!("Cannot find existing feed for {}", xmlurl);
                }
            }
        }
    }
}

fn traverse_and_modify<F>(element: &mut Element, modifier: &mut F)
where
    F: FnMut(&mut Element),
{
    // Modify the current element
    modifier(element);

    // Recursively modify child elements
    for child in element.children.iter_mut() {
        if let Some(child_element) = child.as_mut_element() {
            traverse_and_modify(child_element, modifier);
        }
    }
}

pub fn read_feeds(filename: &str) -> Result<Vec<(String, String)>, String> {
    let file_content = std::fs::read_to_string(filename)
        .map_err(|e| format!("Cannot read feeds file {}: {}", filename, e))?;
    let deserialized: Data = serde_json::from_str(&file_content)
        .map_err(|e| format!("Cannot deserialize feeds file {}: {}", filename, e))?;
    Ok(deserialized.0)
}

impl OpmlDom {
    pub fn new(filename: &str) -> Result<Self, String> {
        info!("Reading OPML file {}", filename);
        let file = File::open(filename)
            .map_err(|e| format!("OPML file {} cannot be opened: {}", filename, e))?;
        let buffered_reader = BufReader::new(file);

        let opmlroot =
            Element::parse(buffered_reader).map_err(|e| format!("XML parse error: {}", e))?;
        Ok(OpmlDom {
            opmlroot,
            filename: filename.to_string(),
            feeds: Vec::new(),
        })
    }

    // must not be called more than once!
    pub fn modify(&mut self, new_url_prefix: String, previous_feeds: &HashMap<String, String>) {
        assert!(self.feeds.is_empty()); // must not be called more than once
        info!(
            "Patching OPML file {} with url prefix {}",
            self.filename, new_url_prefix
        );
        let mut modifier = |element: &mut Element| {
            modify_text_title_and_xmlurl_and_collect_changes(
                element,
                new_url_prefix.clone(),
                &mut self.feeds,
                previous_feeds,
            )
        };
        traverse_and_modify(&mut self.opmlroot, &mut modifier);
    }

    pub fn save_feeds(&mut self, filename: &str) -> Result<(), String> {
        info!("Writing feeds json file {}", filename);
        let data = Data(self.feeds.clone());
        let serialized = serde_json::to_string_pretty(&data).unwrap();
        std::fs::write(filename, serialized).map_err(|e| format!("Cannot write feeds: {}", e))?;
        Ok(())
    }

    pub fn write(&self, filename: &str) -> Result<(), String> {
        info!("Writing OPML file {}", filename);
        let config = EmitterConfig::new()
            .indent_string("    ")
            .line_separator("\n")
            .perform_indent(true)
            .normalize_empty_elements(false);
        self.opmlroot
            .write_with_config(File::create(filename).unwrap(), config)
            .map_err(|e| format!("OPML file {} cannot be written: {}", filename, e))
    }
}

#[cfg(test)]
mod tests {
    use super::super::utilities::setup_test_logger;
    use super::*;

    #[test]
    fn test_opml_ctor() {
        setup_test_logger();
        let opml = OpmlDom::new("testdata/feedly-source.opml");
        assert!(opml.is_ok());
    }

    #[test]
    fn test_traverse_and_modify() {
        setup_test_logger();
        let mut opml = OpmlDom::new("testdata/feedly-source.opml").unwrap();
        let mut collector = Vec::new();
        let previousfeeds = HashMap::new();
        let mut modifier = |element: &mut Element| {
            modify_text_title_and_xmlurl_and_collect_changes(
                element,
                "http://replace.with.my.domain/rssfeeds/".to_string(),
                &mut collector,
                &previousfeeds,
            )
        };
        traverse_and_modify(&mut opml.opmlroot, &mut modifier);
        let result = opml.write("testdata/feedly-target.opml");
        assert!(result.is_ok());
        assert_eq!(collector.len(), 56);
        assert!(std::fs::remove_file("testdata/feedly-target.opml").is_ok());
    }

    #[test]
    fn test_modify_and_save_feeds_with_read() {
        setup_test_logger();
        let mut opml = OpmlDom::new("testdata/feedly-source.opml").unwrap();
        let previousfeeds = HashMap::new();
        opml.modify(
            "http://replace.with.my.domain/rssfeeds/".to_string(),
            &previousfeeds,
        );
        let result = opml.save_feeds("testdata/feeds.json");
        assert!(result.is_ok());
        let feeds = read_feeds("testdata/feeds.json");
        assert!(feeds.is_ok());
        assert_eq!(feeds.unwrap().len(), 56);
    }

    #[test]
    fn test_read_and_write() {
        setup_test_logger();
        let opml = OpmlDom::new("testdata/feedly-source.opml").unwrap();
        let result = opml.write("testdata/new-feedly.opml");
        assert!(result.is_ok());
        let opml2 = OpmlDom::new("testdata/new-feedly.opml");
        assert!(opml2.is_ok());
        assert!(std::fs::remove_file("testdata/new-feedly.opml").is_ok());
    }
}
