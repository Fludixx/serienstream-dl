use rand::prelude::*;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Email {
    pub address: String,
    pub domain: String,
}

impl ToString for Email {
    fn to_string(&self) -> String {
        format!("{}@{}", self.address, self.domain)
    }
}

impl Email {
    pub fn new_from_str(address: String) -> Email {
        let split: Vec<&str> = address.split("@").collect();
        Email {
            address: split[0].to_string(),
            domain: split[1].to_string(),
        }
    }

    pub fn new_from_time() -> Email {
        let raw = reqwest::get("https://api4.temp-mail.org/request/domains/format/json")
            .unwrap()
            .text()
            .unwrap();
        let mut domains: Vec<String> = serde_json::from_str(raw.as_str()).unwrap();
        domains.shuffle(&mut rand::thread_rng());
        Email::new_from_str(format!(
            "{}{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            domains[0],
        ))
    }

    pub fn md5(&self) -> String {
        format!("{:x}", md5::compute(self.to_string()))
    }

    pub fn get_email(&self) -> Option<String> {
        println!("[*] Fetching email from: {}", self.to_string());
        let raw = reqwest::get(
            format!(
                "https://api4.temp-mail.org/request/mail/id/{}/format/json",
                self.md5()
            )
            .as_str(),
        )
        .unwrap()
        .text()
        .unwrap();
        let v: Value = serde_json::from_str(raw.as_str()).unwrap();
        let email = v[0]["mail_text_only"].as_str();
        match email {
            None => None,
            Some(s) => Some(s.to_string().clone()),
        }
    }
}
