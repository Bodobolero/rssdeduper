use super::ids::{convert_url_to_unique_filename, generate_uuid};

use serde::{Deserialize, Serialize};
use serde_json;
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

fn traverse_element(element: &Element) {
    for child in &element.children {
        if let Some(child) = child.as_element() {
            traverse_element(child);
        }
    }
}

fn modify_text_title_and_xmlurl_and_collect_changes(
    element: &mut Element,
    new_url_prefix: String,
    collector: &mut Vec<(String, String)>,
) {
    if element.name == "outline" {
        if let Some(title) = element.attributes.get_mut("title") {
            title.insert_str(0, "DD_");
        }
        if let Some(text) = element.attributes.get_mut("text") {
            text.insert_str(0, "DD_");
        }
        if let Some(xmlurl) = element.attributes.get_mut("xmlUrl") {
            let old_xmlurl = xmlurl.clone();
            let new_filename = convert_url_to_unique_filename(xmlurl, &generate_uuid());
            *xmlurl = new_url_prefix + new_filename.as_str();
            collector.push((old_xmlurl, new_filename));
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
    pub fn modify(&mut self, new_url_prefix: String) {
        assert!(self.feeds.is_empty()); // must not be called more than once
        let mut modifier = |element: &mut Element| {
            modify_text_title_and_xmlurl_and_collect_changes(
                element,
                new_url_prefix.clone(),
                &mut self.feeds,
            )
        };
        traverse_and_modify(&mut self.opmlroot, &mut modifier);
    }

    pub fn save_feeds(&mut self, filename: &str) -> Result<(), String> {
        let data = Data(self.feeds.clone());
        let serialized = serde_json::to_string_pretty(&data).unwrap();
        std::fs::write(filename, serialized).map_err(|e| format!("Cannot write feeds: {}", e))?;
        Ok(())
    }

    pub fn write(&self, filename: &str) -> Result<(), String> {
        let config = EmitterConfig::new()
            .indent_string("    ")
            .line_separator("\n")
            .perform_indent(true)
            .normalize_empty_elements(false);
        self.opmlroot
            .write_with_config(File::create(filename).unwrap(), config)
            .map_err(|e| format!("OPML file {} cannot be written: {}", filename, e))
    }

    // return tuples of (rssfeedurl, rssfeednewfilename)
    // pub fn get_feeds(&self, targetdirectory: &Path) -> Vec<(String, String)> {
    //     let mut feeds = Vec::new();
    //     for outline in self.opmlroot.get_child("body").unwrap().children() {
    //         let mut outline = Outline::new(outline);
    //         if outline.is_feed() {
    //             outlines.push(outline);
    //         }
    //     }
    //     outlines
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opml_ctor() {
        let opml = OpmlDom::new("testdata/feedly-source.opml");
        assert!(opml.is_ok());
    }
    #[test]
    fn test_traverse() {
        let opml = OpmlDom::new("testdata/feedly-source.opml").unwrap();
        traverse_element(&opml.opmlroot);
    }

    #[test]
    fn test_traverse_and_modify() {
        let mut opml = OpmlDom::new("testdata/feedly-source.opml").unwrap();
        let mut collector = Vec::new();
        let mut modifier = |element: &mut Element| {
            modify_text_title_and_xmlurl_and_collect_changes(
                element,
                "http://replace.with.my.domain/rssfeeds/".to_string(),
                &mut collector,
            )
        };
        traverse_and_modify(&mut opml.opmlroot, &mut modifier);
        let result = opml.write("testdata/feedly-with-new-attribute.opml");
        assert!(result.is_ok());
        assert_eq!(collector.len(), 56);
        //print!("urlchanges: {:?}", collector);
    }

    #[test]
    fn test_modify_and_save_feeds_with_read() {
        let mut opml = OpmlDom::new("testdata/feedly-source.opml").unwrap();
        opml.modify("http://replace.with.my.domain/rssfeeds/".to_string());
        let result = opml.save_feeds("testdata/feeds.json");
        assert!(result.is_ok());
        let feeds = read_feeds("testdata/feeds.json");
        assert!(feeds.is_ok());
        assert_eq!(feeds.unwrap().len(), 56);
    }

    #[test]
    fn test_read_and_write() {
        let opml = OpmlDom::new("testdata/feedly-source.opml").unwrap();
        let result = opml.write("testdata/new-feedly.opml");
        assert!(result.is_ok());
        let opml2 = OpmlDom::new("testdata/new-feedly.opml");
        assert!(opml2.is_ok());
        assert!(std::fs::remove_file("testdata/new-feedly.opml").is_ok());
    }
}
