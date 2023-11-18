use super::opml::*;
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// no longer needed after preserving uuids
fn _remove_rss_files(directory: &str) -> Result<(), String> {
    let dir = Path::new(directory);
    if dir.is_dir() {
        for entry in fs::read_dir(dir)
            .map_err(|e| format!("Cannot read feeds directory {}: {}", directory, e))?
        {
            let entry =
                entry.map_err(|e| format!("Cannot read feeds directory {}: {}", directory, e))?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rss") {
                fs::remove_file(path).map_err(|e| {
                    format!("Cannot remove feeds from directory {}: {}", directory, e)
                })?;
            }
        }
    }
    Ok(())
}

// we now want to preserve existing feed uuids because
// - users may have multiple newsreader clients using the same opml file but gradually upgrade them
//   one by one after a new feed was added
// - users may forget to update a newsreader client and still want to see existing deduplicated feeds
//   after a change to the OPML file
pub fn check_and_init_feeds(
    opmlfile: &str,
    feedfile: &str,
    urlprefix: &str,
    newopmlfile: &str,
) -> Result<Vec<(String, String)>, String> {
    if do_we_need_new_json_feeds_file(feedfile, opmlfile).unwrap() {
        let previous_feeds: HashMap<String, String> = read_feeds(feedfile)
            .unwrap_or_default()
            .into_iter()
            // use the feedfile as key and not the xmlurl
            .map(|(v, k)| (k, v))
            .collect();
        if !previous_feeds.is_empty() {
            info!(
                "Trying to preserve uuids of {} previous feeds",
                previous_feeds.len()
            );
        }
        let mut opml = OpmlDom::new(opmlfile)?;
        opml.modify(urlprefix.to_string(), &previous_feeds);
        opml.write(newopmlfile)?;
        opml.save_feeds(feedfile)?;
        // note that this requires the user to pick up the new OPML file to see the new feeds
        error!("A new OPML file {} has been generated\nTo see the new feeds you need to re-import the new OPML file into your newsreader", 
        newopmlfile);
    }
    read_feeds(feedfile)
}

// if the json feeds file does not exist we want to create it
// if the opml file is newer than the json feeds file we want to recreate it
// if the opml file is older than the json feeds file we want to read the json feeds file
// Return true if jsonfile is older than opmlfile or jsonfile does not exist
// need to pass opml file as file2 and json feeds file as files 1
fn do_we_need_new_json_feeds_file(jsonfile: &str, opmlfile: &str) -> std::io::Result<bool> {
    let file1_exists = Path::new(jsonfile).exists();
    let file2_exists = Path::new(opmlfile).exists();
    if file1_exists && file2_exists {
        let metadata1 = fs::metadata(jsonfile)?;
        let metadata2 = fs::metadata(opmlfile)?;

        let modified1 = metadata1.modified()?;
        let modified2 = metadata2.modified()?;

        Ok(modified1 < modified2)
    } else if file2_exists {
        Ok(true)
    } else {
        panic!("OPML source file {} not found", opmlfile);
    }
}

// set up logger for tests with level info
#[cfg(test)]
pub fn setup_test_logger() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter(None, log::LevelFilter::Info)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_is_file1_older_than_file2() -> std::io::Result<()> {
        // Erstellen Sie zwei temporäre Dateien für den Test
        let mut file1 = std::env::temp_dir();
        file1.push("file1.txt");
        let mut file2 = std::env::temp_dir();
        file2.push("file2.txt");

        // cleanup temporary files
        let _ = fs::remove_file(&file1); // ignore errors in case test is run for the first time
        let _ = fs::remove_file(&file2);

        // Schreiben Sie etwas in die Dateien, um sie zu erstellen
        fs::write(&file1, "File1")?;
        fs::write(&file2, "File2")?;

        std::thread::sleep(std::time::Duration::from_secs(1));

        // Schreiben Sie erneut in die zweite Datei, um ihr Änderungsdatum zu aktualisieren
        fs::write(&file2, "File2 updated")?;

        // Testen Sie die Funktion
        assert!(
            do_we_need_new_json_feeds_file(file1.to_str().unwrap(), file2.to_str().unwrap())
                .unwrap()
        );

        assert!(
            !do_we_need_new_json_feeds_file(file2.to_str().unwrap(), file1.to_str().unwrap())
                .unwrap()
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_is_file1_older_than_file2_when_file1_does_not_exist() -> std::io::Result<()> {
        let mut file1 = std::env::temp_dir();
        file1.push("file1.txt");
        let mut file2 = std::env::temp_dir();
        file2.push("file2.txt");

        // cleanup temporary files
        let _ = fs::remove_file(&file1); // ignore errors in case test is run for the first time
        let _ = fs::remove_file(&file2);

        fs::write(&file2, "File2")?;

        assert!(
            do_we_need_new_json_feeds_file(file1.to_str().unwrap(), file2.to_str().unwrap())
                .unwrap()
        );

        Ok(())
    }
    #[test]
    #[should_panic]
    #[serial]
    fn test_is_file1_older_than_file2_when_file2_does_not_exist() {
        // Erstellen Sie zwei temporäre Dateien für den Test
        let mut file1 = std::env::temp_dir();
        file1.push("file1.txt");
        let mut file2 = std::env::temp_dir();
        file2.push("file2.txt");

        // cleanup temporary files
        let _ = fs::remove_file(&file1); // ignore errors in case test is run for the first time
        let _ = fs::remove_file(&file2);

        assert!(fs::write(&file1, "File1").is_ok());

        // should panic because opml file does not exist
        let _ = do_we_need_new_json_feeds_file(file1.to_str().unwrap(), file2.to_str().unwrap());
    }

    // the following test verifies that 55 existing deduped feeds are preserved as is
    // and a new one is added from a new source opml file
    #[test]
    #[serial]
    fn test_check_and_init_feeds() {
        setup_test_logger();

        // prepare files for iteration 1
        let mut feedsfile = std::env::temp_dir();
        feedsfile.push("feeds.json");
        fs::copy("testdata/feeds_iteration1.json", &feedsfile).unwrap();
        // ensure opml file is newer than feeds file
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mut source_opml = std::env::temp_dir();
        source_opml.push("feedly-source.opml");
        fs::copy("testdata/feedly-source_iteration2.opml", &source_opml).unwrap();

        let feeds = check_and_init_feeds(
            source_opml.to_str().unwrap(),
            feedsfile.to_str().unwrap(),
            "https://www.bodobolero.com/rss/",
            "testdata/feedly-target_iteration2.opml",
        );
        assert!(feeds.is_ok());
        assert_eq!(feeds.unwrap().len(), 56);
        // cleanup test files
        let _ = fs::remove_file(&source_opml);
        let _ = fs::remove_file(&feedsfile);
    }
}
