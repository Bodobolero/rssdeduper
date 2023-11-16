use super::opml::*;
use log::error;
use std::fs;
use std::path::Path;

fn remove_rss_files(directory: &str) -> Result<(), String> {
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

pub fn check_and_init_feeds(
    opmlfile: &str,
    feedfile: &str,
    urlprefix: &str,
    newopmlfile: &str,
    targetdirectory: &str,
) -> Result<Vec<(String, String)>, String> {
    if do_we_need_new_json_feeds_file(feedfile, opmlfile).unwrap() {
        let mut opml = OpmlDom::new(opmlfile)?;
        opml.modify(urlprefix.to_string());
        opml.write(newopmlfile)?;
        opml.save_feeds(feedfile)?;
        // now remove all existing feeds from targetdirectory they have old names no longer
        // referenced by the OPML file - note that this requires the user to pick up the new OPML file
        error!("Removing all feed files from {}\nYou need to import the new OPML file {} into your newsreader", 
        targetdirectory, newopmlfile);
        remove_rss_files(targetdirectory)?;
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

        // Warten Sie einen Moment, um sicherzustellen, dass die Dateien unterschiedliche Änderungsdaten haben
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
        // Erstellen Sie zwei temporäre Dateien für den Test
        let mut file1 = std::env::temp_dir();
        file1.push("file1.txt");
        let mut file2 = std::env::temp_dir();
        file2.push("file2.txt");

        // cleanup temporary files
        let _ = fs::remove_file(&file1); // ignore errors in case test is run for the first time
        let _ = fs::remove_file(&file2);

        fs::write(&file2, "File2")?;

        // Testen Sie die Funktion
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
}
