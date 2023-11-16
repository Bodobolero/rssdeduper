extern crate test;

use lazy_static::lazy_static;
use regex::Regex;
use url::Url;
use uuid::Uuid;

lazy_static! {
    // UUID
    static ref UUID_REGEX: Regex = Regex::new(
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}"
    ).unwrap();
    // number with at least 6 digits
    static ref NUMBER_REGEX: Regex = Regex::new(r"[0-9_]{6}[0-9_]*").unwrap();

    static ref SANITIZE_REGEX: Regex = Regex::new(r"[^a-zA-Z0-9]+").unwrap();
}

pub fn extract_unique_id_and_host_from_url_string(url: &str) -> Option<(String, String)> {
    let parsed_url = Url::parse(url).ok()?;
    let host = parsed_url.host().unwrap().to_string();
    let mut id = url;
    if let Some(cap) = UUID_REGEX.captures(parsed_url.path()) {
        if let Some(uuid_str) = cap.get(0) {
            if Uuid::parse_str(uuid_str.as_str()).is_ok() {
                id = uuid_str.as_str();
            }
        }
    } else if let Some(cap) = NUMBER_REGEX.captures(parsed_url.path()) {
        if let Some(id_str) = cap.get(0) {
            id = id_str.as_str();
        }
    }
    Some((id.to_string(), host))
}

fn convert_url_to_filename(url: &str) -> String {
    SANITIZE_REGEX.replace_all(url, "_").to_string() + ".rss"
}

pub fn generate_uuid() -> String {
    let uuid = Uuid::new_v4(); // Generate a random UUID
    uuid.to_string() // Convert it to a string
}

pub fn convert_url_to_unique_filename(url: &str, uuid: &str) -> String {
    uuid.to_owned() + convert_url_to_filename(url).as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn extract_faz() {
        let url = "https://www.faz.net/aktuell/finanzen/zinssaetze-fuer-festgeld-warum-erste-banken-die-sparzinsen-wieder-senken-19313464.html";
        assert_eq!(
            extract_unique_id_and_host_from_url_string(url).unwrap(),
            (String::from("19313464"), String::from("www.faz.net"))
        );
    }

    #[test]
    fn extract_stz() {
        let url = "https://www.stuttgarter-zeitung.de/inhalt.gluehwein-djs-und-handgemachte-geschenke-kleine-und-alternative-weihnachtsmaerkte-in-stuttgart.f3d6053d-c298-4b83-8e70-d5d6e7e8ed78.html";
        assert_eq!(
            extract_unique_id_and_host_from_url_string(url).unwrap(),
            (
                String::from("f3d6053d-c298-4b83-8e70-d5d6e7e8ed78"),
                String::from("www.stuttgarter-zeitung.de")
            )
        );
    }

    #[test]
    fn extract_elpais() {
        let url = "https://elviajero.elpais.com/elviajero/2022/07/26/actualidad/1658829008_842300.html#?ref=rss&format=simple&link=link
        ";
        assert_eq!(
            extract_unique_id_and_host_from_url_string(url).unwrap(),
            (
                String::from("1658829008_842300"),
                String::from("elviajero.elpais.com")
            )
        );
    }

    #[test]
    fn test_convert_url_to_filename() {
        let url = "https://www.faz.net/aktuell/finanzen/";
        assert_eq!(
            convert_url_to_filename(url),
            String::from("https_www_faz_net_aktuell_finanzen_.rss")
        );
    }

    #[test]
    fn test_generate_uuid() {
        let uuid = generate_uuid();
        assert!(UUID_REGEX.captures(&uuid).is_some());
        assert_eq!(uuid.len(), 36);
    }

    #[test]
    fn test_convert_url_to_unique_filename() {
        let url = "https://www.faz.net/aktuell/finanzen/";
        let uuid = generate_uuid();
        assert_eq!(
            convert_url_to_unique_filename(url, &uuid),
            uuid + "https_www_faz_net_aktuell_finanzen_.rss"
        );
    }

    #[bench]
    fn bench_extract_unique_id_and_host_from_url_string(b: &mut Bencher) {
        let url = "https://elviajero.elpais.com/elviajero/2022/07/26/actualidad/1658829008_842300.html#?ref=rss&format=simple&link=link
        ";
        b.iter(|| {
            extract_unique_id_and_host_from_url_string(url);
        })
    }
}
