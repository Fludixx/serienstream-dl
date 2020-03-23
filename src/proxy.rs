use rand::prelude::SliceRandom;
use std::error::Error;
use std::str::FromStr;

pub struct HttpsProxy {
    pub address: String,
    pub port: u16,
}

impl HttpsProxy {
    pub fn new() -> Result<HttpsProxy, Box<dyn Error>> {
        let site = reqwest::get("https://api.proxyscrape.com/?request=getproxies&proxytype=http&timeout=10000&country=all&ssl=yes&anonymity=all")?.text()?;
        let mut site_array: Vec<&str> = site.split("\r\n").collect();
        if site_array.len() < 2 {
            Err("Invalid resposne from Proxyscrape")?
        }
        site_array.shuffle(&mut rand::thread_rng());
        let address = site_array.get(0);
        if address.is_none() {
            Err("Invalid resposne from Proxyscrape")?
        }
        let address: Vec<&str> = address.unwrap().split(":").collect();
        let ip = address.get(0);
        let port = address.get(1);
        if ip.is_none() || port.is_none() {
            Err("Invalid resposne from Proxyscrape")?
        }
        let ip = ip.unwrap();
        let port = u16::from_str(port.unwrap())?;
        Ok(HttpsProxy {
            address: ip.to_string(),
            port,
        })
    }
}
