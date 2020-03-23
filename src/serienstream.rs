use crate::email::Email;
use crate::proxy::HttpsProxy;
use colored::Colorize;
use regex::Regex;
use reqwest::cookie::Cookie;
use reqwest::Proxy;
use serde_json::Value;
use std::borrow::Borrow;
use std::error::Error;
use std::thread;
use std::time::Duration;

// I got the token from the android app :P
pub const KEY: &str = "9bmkkkvloi4o10pnel886l1xj6ztycualnmofbkrsfzsmc26lrujoesptp8aqw";
// also from the android app, but they do have documentation on how to use the api, but I don't know
// how complete this documentation is.
pub const SITE: &str = "https://s.to";
pub const ENDPOINT: &str = "https://s.to/api/v1/";

#[derive(Clone, PartialEq)]
#[repr(u8)]
pub enum Language {
    German = 1,
    GermanSubtitles = 2,
    English = 3,
    Unknown = 0,
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
    pub series: Series,
    pub id: u32,
    body: Value,
}

#[derive(Clone)]
pub struct Episode {
    pub season: Season,
    pub id: u32,
    body: Value,
}

#[derive(Clone)]
pub struct StreamHost {
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

#[derive(Clone)]
pub struct Url {
    pub episode: Episode,
    pub host: Host,
    pub streamer_url: String,
}

#[derive(Clone, Debug)]
pub enum Host {
    Vivo,
    Vidoza,
    Voe,
    GoUnlimited,
    JetLoad,
    UpStream,
    VidLox,
    Unknown,
}

impl Host {
    pub fn from_str(str: &str) -> Self {
        match str {
            "Vivo" => Host::Vivo,
            "Vidoza" => Host::Vidoza,
            "VOE" => Host::Voe,
            "GoUnlimited" => Host::GoUnlimited,
            "JetLoad" => Host::JetLoad,
            "UpStream" => Host::UpStream,
            "VidLox" => Host::VidLox,
            _ => Host::Unknown,
        }
    }
}

impl Account {
    pub fn from_str(v: &str) -> Result<Account, Box<dyn Error>> {
        let v = v.replace("\r", "").replace("\n", "");
        let creds: Vec<&str> = v.split(":").collect();
        if creds.len() < 2 {
            Err("Invalid account entry")?
        }
        Ok(Account {
            name: String::from("nameless"),
            email: Email::new_from_str(creds[0].to_string()),
            password: creds[1].to_string(),
        })
    }

    pub fn create(name: String, email: Email, password: String) -> Result<Account, Box<dyn Error>> {
        let proxy_info = HttpsProxy::new();
        if proxy_info.is_err() {
            println!(
                "{}",
                "Unable to get proxy. Waiting 2 seconds before retrying...".yellow()
            );
            thread::sleep(Duration::from_secs(2));
            return Account::create(name, email, password);
        }
        let proxy_info = proxy_info?;
        println!(
            "{}Using proxy: {}...",
            format!("[Thread:{}]", thread::current().name().unwrap_or("?"))
                .as_str()
                .bright_purple(),
            format!("{}:{}", proxy_info.address, proxy_info.port)
                .as_str()
                .bright_blue()
        );
        let proxy =
            Proxy::https(format!("http://{}:{}", proxy_info.address, proxy_info.port).as_str());
        if proxy.is_err() {
            return Account::create(name, email, password);
        }
        let proxy = proxy?;
        let c = reqwest::Client::builder().proxy(proxy).build()?;
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
                "Mozilla/5.0 (X11; Linux x86_64; rv:74.0) Gecko/20100101 Firefox/74.0",
            )
            .form(&params)
            .send();
        if r.is_err() {
            return Account::create(name, email, password);
        }
        let site = r?.text()?;
        let validation_regex = Regex::new(
            r#"Dein Account wurde erfolgreich erstellt\. Um die Registrierung abzuschließen, bestätige bitte deine E-Mail-Adresse durch die an dich gesendete Mail"#,
        )?;
        if !validation_regex.is_match(&site) {
            return Account::create(name, email, password);
        }
        let mut looped: u16 = 0;
        loop {
            looped += 1;
            println!(
                "{}Fetching emails of: {}",
                format!("[Thread:{}]", thread::current().name().unwrap_or("?"))
                    .as_str()
                    .bright_purple(),
                email.to_string().as_str().bright_blue()
            );
            match email.get_email() {
                Err(_) => {
                    if looped > 30 {
                        // after 1min cancel
                        println!("{}", "Skipping after 1 minute without email.".yellow());
                        Err("Email timeout")?
                    }
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
                Ok(text) => {
                    if text.is_none() {
                        continue;
                    }
                    let text = text.unwrap();
                    let r =
                        Regex::new(r#"(https://s.to/registrierung/\?verification=[a-zA-Z0-9]+)""#)?;
                    let capture = r.captures(text.as_str()).unwrap();
                    let url = capture.get(1).unwrap().as_str();
                    println!(
                        "{}{}",
                        format!("[Thread:{}]", thread::current().name().unwrap_or("?"))
                            .as_str()
                            .bright_purple(),
                        "Finishing Account creation...".yellow()
                    );
                    reqwest::get(url)?;
                    return Ok(Account {
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

    pub fn from_name(name: &str) -> Result<Series, Box<dyn Error>> {
        Series::from_url(
            format!(
                "{}/serie/stream/{}",
                SITE,
                name.to_ascii_lowercase().replace(" ", "-")
            )
            .as_str(),
        )
    }

    pub fn from_url(url: &str) -> Result<Series, Box<dyn Error>> {
        let r = reqwest::get(url)?.text()?;
        let id_regex = Regex::new("series-id=\"(\\d+)\"")?;
        let result = id_regex
            .captures(r.as_str())
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        Ok(Series {
            id: result.parse::<u32>()?,
        })
    }

    pub fn get_season(&self, season: u32) -> Result<Season, Box<dyn Error>> {
        Season::new_from_id(self.id, season)
    }

    pub fn get_season_count(&self) -> u32 {
        let mut i = 1;
        loop {
            let r = reqwest::get(
                format!(
                    "{}series/get?series={}&season={}&key={}",
                    ENDPOINT, self.id, i, KEY
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
    pub fn new_from_id(series_id: u32, id: u32) -> Result<Season, Box<dyn Error>> {
        Ok(Season {
            series: Series { id: series_id },
            id,
            body: serde_json::from_str(
                reqwest::get(
                    format!(
                        "{}series/get?series={}&season={}&key={}",
                        ENDPOINT, series_id, id, KEY
                    )
                    .as_str(),
                )?
                .text()?
                .as_str(),
            )?,
        })
    }

    pub fn get_series(&self) -> Series {
        self.series.clone()
    }

    pub fn get_link(&self) -> String {
        format!(
            "{}/serie/stream/{}",
            SITE,
            self.body["series"]["link"].as_str().unwrap()
        )
    }

    pub fn get_name(&self) -> String {
        String::from(self.body["series"]["name"].as_str().unwrap())
    }

    pub fn get_episode(&self, episode: u32) -> Result<Episode, Box<dyn Error>> {
        if self.body["episodes"].get(episode as usize).is_none() {
            Err("Episode not found")?
        }
        Ok(Episode {
            season: self.clone(),
            id: episode,
            body: self.body["episodes"][episode as usize].clone(),
        })
    }

    pub fn get_episode_count(&self) -> usize {
        self.body["episodes"].as_array().unwrap().len()
    }
}

impl Episode {
    pub fn get_link(&self) -> String {
        format!(
            "{}/serie/stream/{}/staffel-{}/episode-{}",
            SITE,
            self.season.body["series"]["link"].as_str().unwrap(),
            self.season.id,
            self.id + 1
        )
    }

    pub fn get_name(&self, language: &Language) -> String {
        let german_name = self.body["german"].as_str().unwrap();
        let english_name = self.body["english"].as_str().unwrap();
        if language == &Language::Unknown
            || ((language == &Language::German || language == &Language::GermanSubtitles)
                && german_name.len() < 1)
        {
            String::from(english_name)
        } else {
            String::from(german_name)
        }
    }

    pub fn get_stream_url(&self, language: &Language) -> Result<StreamHost, Box<dyn Error>> {
        let streamers = self.body["links"].clone();
        if !streamers.is_array() {
            println!("{}", "No streamers available. Skipping...".red());
            Err("No streamers available")?
        }
        let mut i = 0;
        let mut streamer: Option<&Value> = None;
        if language != &Language::Unknown {
            while streamers.get(i).is_some() {
                let potential_streamer = streamers.get(i).unwrap();
                if Language::from_number(potential_streamer["language"].as_i64().unwrap())
                    == *language
                {
                    streamer = Some(potential_streamer);
                    break;
                }
                i += 1;
            }
            if streamer.is_none() {
                println!(
                    "{}",
                    "Couldn't find episode with specific Language. Skipping...".red()
                );
                Err("Episode with specific language is not available")?
            }
        } else {
            streamer = Some(&streamers[0]);
        }
        let streamer = streamer.unwrap();
        let link = streamer["link"].as_str();
        if link.is_none() {
            Err("Malformed api response?")?;
        }
        let id_regex = Regex::new(r#"\d{2,9}"#)?;
        let id = id_regex.find(link.unwrap()).unwrap().as_str();
        Ok(StreamHost {
            name: streamer["hoster"].as_str().unwrap().to_string(),
            url: format!("{}/redirect/{}", SITE, id),
            language: Language::from_number(streamer["language"].as_i64().unwrap()),
            episode: self.clone(),
        })
    }
}

impl StreamHost {
    pub fn get_site_url(&self, acc: &Account) -> Result<Url, Box<dyn Error>> {
        let email = acc.email.to_string();
        let password = &acc.password;
        let params = [
            ("email", email.as_str()),
            ("password", password.as_str()),
            ("autoLogin", "on"),
        ];
        let login = reqwest::Client::new()
            .post(format!("{}/login", SITE).as_str())
            .form(&params)
            .send()?;
        let cookies: Vec<Cookie> = login.cookies().collect();
        let mut login_key = String::new();
        for cookie in cookies {
            if cookie.name() == "rememberLogin" {
                login_key = cookie.value().to_string();
                break;
            }
        }
        if login_key.len() < 2 {
            return self.get_site_url(&acc);
        }
        let response = reqwest::Client::new()
            .post(self.url.as_str())
            .header("Cookie", format!("rememberLogin={}", login_key).as_str())
            .send();
        if response.is_err() {
            println!("{}", "Failed to resolve link.".yellow());
            return self.get_site_url(&acc);
        }
        let response = response?;
        let url = response.url().as_str();
        if url.contains(SITE) {
            return self.get_site_url(&acc);
        }
        println!(
            "Resolved real location of: Season: {}, Episode: {}",
            self.episode.season.id,
            self.episode.id + 1
        );
        reqwest::Client::new()
            .get(format!("{}/logout", SITE).as_str())
            .header("Cookie", format!("rememberLogin={}", login_key).as_str())
            .send()?;
        Ok(Url {
            episode: self.episode.clone(),
            host: Host::from_str(self.name.as_str()),
            streamer_url: String::from(url),
        })
    }
}
