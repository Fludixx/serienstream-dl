use colored::Colorize;
use rand::prelude::SliceRandom;
use std::process::exit;
use std::str::FromStr;

pub struct HttpsProxy {
    pub address: String,
    pub port: u16,
}

impl HttpsProxy {
    pub fn new() -> HttpsProxy {
        let site = reqwest::get("https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=10000&country=all&ssl=yes&anonymity=all").unwrap().text().unwrap();
        let mut site_array: Vec<&str> = site.split("\r\n").collect();
        if site_array.len() < 2 {
            println!("{}", "Reached hourly limit of Proxyscrape.".red());
            exit(1);
        }
        site_array.shuffle(&mut rand::thread_rng());
        let ip: Vec<&str> = site_array[0].split(":").collect();
        HttpsProxy {
            address: ip[0].to_string(),
            port: u16::from_str(ip[1]).unwrap(),
        }
    }
}
