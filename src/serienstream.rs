use crate::email::Email;
use crate::proxy::HttpsProxy;
use regex::Regex;
use reqwest::cookie::Cookie;
use reqwest::Proxy;
use serde_json::Value;
use std::thread;
use std::time::Duration;

pub const TOKEN: &str = "9bmkkkvloi4o10pnel886l1xj6ztycualnmofbkrsfzsmc26lrujoesptp8aqw";
pub const ENDPOINT: &str = "https://s.to/api/v1/";
pub const SITE: &str = "https://s.to";

#[derive(Clone)]
pub enum Language {
    German = 1,
    GermanSubtitles = 2,
    English = 3,
    Unknown,
}

impl Language {
    pub fn from_number(i: i64) -> Language {
        match i {
            1 => Language::German,
            2 => Language::GermanSubtitles,
            3 => Language::English,
            _ => Language::Unknown,
        }
    }
}

#[derive(Clone)]
pub struct Series {
    pub id: u32,
}

#[derive(Clone)]
pub struct Season {
    pub id: u32,
    pub season: u32,
}

#[derive(Clone)]
pub struct Episode {
    pub id: u32,
    pub season: u32,
    pub episode: u32,
}

#[derive(Clone)]
pub struct StreamHoster {
    pub name: String,
    pub url: String,
    pub language: Language,
    pub episode: Episode,
}

#[derive(Clone)]
pub struct Account {
    pub name: String,
    pub email: Email,
    pub password: String,
}

impl Account {
    pub fn from_str(v: &str) -> Account {
        let v = v.replace("\r", "").replace("\n", "");
        let creds: Vec<&str> = v.split(":").collect();
        Account {
            name: String::from("nameless"),
            email: Email::new_from_str(creds[0].to_string()),
            password: creds[1].to_string(),
        }
    }

    pub fn create(name: String, email: Email, password: String) -> Option<Account> {
        let proxy_info = HttpsProxy::new();
        println!(
            "[*] Using proxy: {}:{}",
            proxy_info.address, proxy_info.port
        );
        let proxy =
            Proxy::https(format!("http://{}:{}", proxy_info.address, proxy_info.port).as_str())
                .unwrap();
        let c = reqwest::Client::builder().proxy(proxy).build().unwrap();
        let params = [
            ("userName", name.clone()),
            ("userPassword1", password.clone()),
            ("userPassword2", password.clone()),
            ("userEmail1", email.clone().to_string()),
        ];
        let r = c
            .post(format!("{}/registrierung", SITE).as_str())
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:71.0) Gecko/20100101 Firefox/71.0",
            )
            .form(&params)
            .send();
        if r.is_err() {
            return Account::create(name, email, password);
        }
        let mut r = r.unwrap();
        let mut looped: u16 = 0;
        loop {
            match email.get_email() {
                None => {
                    if looped > 14 {
                        // after 30s cancel
                        return None;
                    }
                    thread::sleep(Duration::from_secs(2));
                    looped += 1;
                    continue;
                }
                Some(text) => {
                    let r =
                        Regex::new(r#"(https://s.to/registrierung/\?verification=[a-zA-Z0-9]+)""#)
                            .unwrap();
                    let url = r.captures(text.as_str()).unwrap().get(1).unwrap().as_str();
                    println!("[*] Finishing Account creation");
                    reqwest::get(url).unwrap();
                    return Some(Account {
                        name,
                        email,
                        password,
                    });
                }
            }
        }
    }
}

impl Series {
    pub fn from_id(id: u32) -> Series {
        Series { id }
    }

    pub fn from_name(name: &str) -> Series {
        Series::from_url(
            format!(
                "{}/serie/stream/{}",
                SITE,
                name.to_ascii_lowercase().replace(" ", "-")
            )
            .as_str(),
        )
    }

    pub fn from_url(url: &str) -> Series {
        let r = reqwest::get(url).unwrap().text().unwrap();
        let id_regex = Regex::new("series-id=\"(\\d+)\"").unwrap();
        let result = id_regex
            .captures(r.as_str())
            .expect("Series not found")
            .get(1)
            .expect("Site Malformed?")
            .as_str();
        Series {
            id: result.parse::<u32>().unwrap(),
        }
    }

    pub fn get_season(&self, season: u32) -> Season {
        Season {
            id: self.id,
            season,
        }
    }

    pub fn get_season_count(&self) -> u32 {
        let mut i = 1;
        loop {
            let r = reqwest::get(
                format!(
                    "{}series/get?series={}&season={}&key={}",
                    ENDPOINT, self.id, i, TOKEN
                )
                .as_str(),
            )
            .unwrap()
            .text()
            .unwrap();
            if r.starts_with("{") {
                // if we get a valid json response the season exists
                i += 1;
            } else {
                break;
            }
        }
        i
    }
}

impl Season {
    pub fn get_series(&self) -> Series {
        Series { id: self.id }
    }

    pub fn get_link(&self) -> String {
        let raw_json = reqwest::get(
            format!(
                "{}series/get?series={}&season={}&key={}",
                ENDPOINT, self.id, self.season, TOKEN
            )
            .as_str(),
        )
        .unwrap()
        .text()
        .unwrap();
        let json: Value = serde_json::from_str(raw_json.as_str()).unwrap();
        format!(
            "{}/serie/stream/{}",
            SITE,
            json["series"]["link"].as_str().unwrap()
        )
    }

    pub fn get_episode(&self, episode: u32) -> Episode {
        Episode {
            id: self.id,
            season: self.season,
            episode,
        }
    }

    pub fn get_episode_count(&self) -> u32 {
        let raw_json = reqwest::get(
            format!(
                "{}series/get?series={}&season={}&key={}",
                ENDPOINT, self.id, self.season, TOKEN
            )
            .as_str(),
        )
        .unwrap()
        .text()
        .unwrap();
        let json: Value = serde_json::from_str(raw_json.as_str()).unwrap();
        let episodes_string: String = json["episodes"].clone().to_string();
        let episode_regex = Regex::new("\"episode\":\\d+").unwrap();
        episode_regex.find_iter(episodes_string.as_str()).count() as u32
    }
}

impl Episode {
    pub fn get_season(&self) -> Season {
        Season {
            id: self.id,
            season: self.season,
        }
    }

    pub fn get_link(&self) -> String {
        let raw_json = reqwest::get(
            format!(
                "{}series/get?series={}&season={}&key={}",
                ENDPOINT, self.id, self.season, TOKEN
            )
            .as_str(),
        )
        .unwrap()
        .text()
        .unwrap();
        let json: Value = serde_json::from_str(raw_json.as_str()).unwrap();
        format!(
            "{}/serie/stream/{}/staffel-{}/episode-{}",
            SITE,
            json["series"]["link"].as_str().unwrap(),
            self.season,
            self.episode + 1
        )
    }

    pub fn get_stream_url(&self) -> StreamHoster {
        let raw_json = reqwest::get(
            format!(
                "{}series/get?series={}&season={}&key={}",
                ENDPOINT, self.id, self.season, TOKEN
            )
            .as_str(),
        )
        .unwrap()
        .text()
        .unwrap();
        let json: Value = serde_json::from_str(raw_json.as_str()).unwrap();
        let streamer = json["episodes"][self.episode as usize]["links"][0].clone();
        let id_regex = Regex::new(r#"\d{2,9}"#).unwrap();
        let id = id_regex
            .find(streamer["link"].as_str().unwrap())
            .unwrap()
            .as_str();
        StreamHoster {
            name: streamer["hoster"].as_str().unwrap().to_string(),
            url: format!("{}/redirect/{}", SITE, id),
            language: Language::from_number(streamer["language"].as_i64().unwrap()),
            episode: Episode {
                id: self.id,
                season: self.season,
                episode: self.episode,
            },
        }
    }
}

impl StreamHoster {
    pub fn get_site_url(&self, acc: Account) -> Option<String> {
        let email = acc.email.to_string();
        let password = acc.password;
        let params = [
            ("email", email.as_str()),
            ("password", password.as_str()),
            ("autoLogin", "on"),
        ];
        let login = reqwest::Client::new()
            .post(format!("{}/login", SITE).as_str())
            .form(&params)
            .send()
            .unwrap();
        let cookies: Vec<Cookie> = login.cookies().collect();
        let mut login_key = String::new();
        for cookie in cookies {
            if cookie.name() == "rememberLogin" {
                login_key = cookie.value().to_string();
                break;
            }
        }
        if login_key.len() < 2 {
            println!("[!] login_key invalid");
            return None;
        }
        println!("[*] Logged in into: {}", acc.email.to_string());
        println!("[*] Resolving real location from: {}", self.url);
        let r = reqwest::Client::new()
            .post(self.url.clone().as_str())
            .header("Cookie", format!("rememberLogin={}", login_key).as_str())
            .send()
            .unwrap();
        let url = r.url().as_str();
        if url.contains("s.to") {
            println!("[!] Account exceeded limit");
            return None;
        }
        println!("[*] Resolved real location: {}", url);
        println!("[*] Logging out");
        reqwest::Client::new()
            .get(format!("{}/logout", SITE).as_str())
            .header("Cookie", format!("rememberLogin={}", login_key).as_str())
            .send();
        Some(url.to_string())
    }
}
