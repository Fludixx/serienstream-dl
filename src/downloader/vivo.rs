use crate::downloader::Downloader;
use crate::serienstream::Host;
use regex::Regex;
use std::error::Error;

fn caesar(input: String, alphabet: &str, shift: i32) -> String {
    let len = alphabet.len();
    let mut out = String::new();
    for c in input.chars() {
        if alphabet.contains(c) {
            out.push(
                alphabet.as_bytes()[((alphabet.find(c).unwrap() + shift as usize) % len)] as char,
            );
        } else {
            out.push(c);
        }
    }
    out
}

fn rot47(input: String) -> String {
    caesar(input, "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~", 47)
}

pub fn new(url: &str) -> Result<Downloader, Box<dyn Error>> {
    let url = url.replace("embed/", "");
    let mut request = reqwest::get(&url).expect("Failed to reach vivo.sx");
    let site_source = request.text().unwrap();
    let source_regex =
        Regex::new(r#"(?s)InitializeStream\s*\(\s*\{.+source:\s*'([A-Za-z0-9%_]+)',"#).unwrap();
    let name_regex =
        Regex::new(r#"(?s)<div\sclass="stream-content"\sdata-name="(.+)"\sdata"#).unwrap();
    let source_capture = source_regex.captures(site_source.as_str());
    let name_capture = name_regex.captures(site_source.as_str());
    if source_capture.is_none() || name_capture.is_none() {
        Err("Failed to retrieve sources")?
    }
    let source_capture = source_capture.unwrap();
    let name_capture = name_capture.unwrap();
    let video_url = rot47(urlencoding::decode(source_capture.get(1).unwrap().as_str()).unwrap());
    let file_name = String::from(name_capture.get(1).unwrap().as_str());

    Ok(Downloader {
        url,
        video_url,
        file_name,
        host: Host::Vivo,
    })
}
