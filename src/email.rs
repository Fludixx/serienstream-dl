use rand::prelude::*;
use serde_json::Value;
use std::error::Error;

pub const ENDPOINT: &str = "https://api4.temp-mail.org/request";

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

    pub fn new_random() -> Result<Email, Box<dyn Error>> {
        let raw = reqwest::get(format!("{}/domains/format/json", ENDPOINT).as_str())?.text()?;
        let mut domains: Vec<String> = serde_json::from_str(raw.as_str()).unwrap();
        domains.shuffle(&mut rand::thread_rng());
        // why a number and not a string?
        // well idk, looks like this email api don't like strings
        let random: u128 = thread_rng().gen();
        Ok(Email::new_from_str(format!("{}{}", random, domains[0])))
    }

    pub fn md5(&self) -> String {
        format!("{:x}", md5::compute(self.to_string()))
    }

    pub fn get_email(&self) -> Result<Option<String>, Box<dyn Error>> {
        let raw =
            reqwest::get(format!("{}/mail/id/{}/format/json", ENDPOINT, self.md5()).as_str())?
                .text()?;
        let v: Value = serde_json::from_str(raw.as_str()).unwrap();
        let email = v[0]["mail_text_only"].as_str();
        match email {
            None => Ok(None),
            Some(s) => Ok(Some(String::from(s))),
        }
    }
}
