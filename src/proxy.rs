use colored::Colorize;
use rand::prelude::SliceRandom;
use std::process::exit;
use std::str::FromStr;

pub struct HttpsProxy {
    pub address: String,
    pub port: u16,
}

impl HttpsProxy {
    pub fn new() -> Option<HttpsProxy> {
        let site = reqwest::get("https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=10000&country=all&ssl=yes&anonymity=all").unwrap().text().unwrap();
        let mut site_array: Vec<&str> = site.split("\r\n").collect();
        if site_array.len() < 2 {
            return None;
        }
        site_array.shuffle(&mut rand::thread_rng());
        let address = site_array.get(0);
        if address.is_none() {
            return None;
        }
        let address: Vec<&str> = address.unwrap().split(":").collect();
        let ip = address.get(0);
        let port = address.get(1);
        if ip.is_none() || port.is_none() {
            return None;
        }
        let ip = ip.unwrap();
        let port = u16::from_str(port.unwrap()).unwrap();
        Some(HttpsProxy {
            address: ip.to_string(),
            port,
        })
    }
}
