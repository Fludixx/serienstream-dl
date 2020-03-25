use crate::downloader::{vidoza, vivo};
use crate::email::Email;
use crate::serienstream::{Account, Host, Language, Series, Url};
use clap::{App, Arg};
use colored::Colorize;
use rand::distributions::Alphanumeric;
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::error::Error;
use std::fs::{create_dir, read_to_string, File, OpenOptions};
use std::io::Write;
use std::process::{exit, Command};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

mod downloader;
mod email;
mod proxy;
mod serienstream;

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => eprintln!("{}", e.description().red()),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let matches = App::new("Serienstream Downloader")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            Arg::with_name("url")
                .long("url")
                .short("u")
                .help("Specifies a source via url")
                .takes_value(true)
                .conflicts_with("name")
                .conflicts_with("id"),
        )
        .arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .help("Specifies a source via name")
                .takes_value(true)
                .conflicts_with("id")
                .conflicts_with("url"),
        )
        .arg(
            Arg::with_name("id")
                .long("id")
                .short("i")
                .help("Specifies a source via id")
                .takes_value(true)
                .conflicts_with("name")
                .conflicts_with("url"),
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .short("o")
                .help("Specifies a folder to save")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("german")
                .long("german")
                .short("g")
                .help("Only downloads german streams")
                .conflicts_with("gersub")
                .conflicts_with("english"),
        )
        .arg(
            Arg::with_name("gersub")
                .long("gersub")
                .short("s")
                .help("Only downloads streams with german subtitles")
                .conflicts_with("german")
                .conflicts_with("english"),
        )
        .arg(
            Arg::with_name("english")
                .long("english")
                .short("e")
                .help("Only downloads english streams")
                .conflicts_with("german")
                .conflicts_with("gersub"),
        )
        .arg(
            Arg::with_name("season")
                .long("season")
                .help("Downloads whole season, --season 1")
                .conflicts_with("episode")
                .conflicts_with("series")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("episode")
                .long("episode")
                .help("Downloads 1 episode, --episode 1,0")
                .conflicts_with("series")
                .conflicts_with("season")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("series")
                .long("series")
                .help("Downloads whole series")
                .conflicts_with("episode")
                .conflicts_with("season"),
        )
        .arg(
            Arg::with_name("generate")
                .long("generate")
                .help("Generates Accounts")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("threads")
                .long("threads")
                .short("t")
                .help("Specify how many threads should be used to generate Accounts")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("generate-name")
                .long("generate-name")
                .help("Let the program decide a name for downloaded files")
                .takes_value(true)
                .default_value("true"),
        )
        .arg(
            Arg::with_name("force-youtube-dl")
                .long("force-youtube-dl")
                .short("f")
                .help("Use youtube-dl to download episodes"),
        )
        .get_matches();

    if matches.is_present("generate") {
        let threads;
        if matches.is_present("threads") {
            threads = u32::from_str(matches.value_of("threads").unwrap()).unwrap();
        } else {
            threads = 1;
        }
        let raw = matches.value_of("generate").unwrap();
        let amount = u32::from_str(raw).unwrap();
        let mut extra_accounts = amount % threads;
        let per_thread = amount / threads;
        if File::open("accounts.txt").is_err() {
            File::create("accounts.txt")?;
        }
        let mut handles = Vec::new();
        for i in 0..threads {
            println!("Starting thread: #{}...", i);
            let builder = thread::Builder::new();
            handles.push(builder.name(format!("{}", i)).spawn(move || {
                generate_account(per_thread + extra_accounts).unwrap();
            })?);
            extra_accounts = 0;
            if i % 10 == 0 {
                // Proxyscrape has a limit of 10 requests/second
                // To be safe we wait 2 seconds
                thread::sleep(Duration::from_secs(2));
            }
        }
        for handle in handles {
            handle.join();
        }
        exit(0);
    }

    let acclist = read_to_string("accounts.txt");
    match acclist {
        Err(_) => {
            println!("Please add some accounts first (--generate)");
            exit(0);
        }
        Ok(s) => {
            if s.len() < 2 {
                println!("Please add some accounts first (--generate)");
                exit(0);
            }
        }
    }

    let s: Series;
    let language: Language;
    let output: String;
    let mut urls: Vec<Url> = Vec::new();
    let generate_name = match matches.value_of("generate-name").unwrap() {
        "true" | "1" | "yes" => true,
        _ => false,
    };
    let use_youtube_dl = matches.is_present("force-youtube-dl");

    if matches.is_present("url") {
        s = Series::from_url(matches.value_of("url").unwrap())?;
    } else if matches.is_present("name") {
        s = Series::from_name(matches.value_of("name").unwrap())?;
    } else if matches.is_present("id") {
        s = Series::from_id(matches.value_of("id").unwrap().parse::<u32>()?);
    } else {
        println!("You need to specify a source");
        exit(0);
    }

    if matches.is_present("german") {
        language = Language::German;
    } else if matches.is_present("english") {
        language = Language::English
    } else if matches.is_present("gersub") {
        language = Language::GermanSubtitles
    } else {
        language = Language::Unknown
    }

    if matches.is_present("output") {
        output = matches.value_of("output").unwrap().to_string();
    } else {
        output = format!("{}", s.id);
    }
    create_dir(output.clone());

    if matches.is_present("episode") {
        let raw = matches.value_of("episode").unwrap();
        let info: Vec<&str> = raw.split(",").collect();
        let season = u32::from_str(info[0])?;
        let episode = u32::from_str(info[1])?;
        if episode == 0 {
            Err("Episodes start at 1")?
        }
        let episode = episode - 1;
        urls.push(download_episode(&s, season, episode, &language)?);
    } else if matches.is_present("season") {
        let raw = matches.value_of("season").unwrap();
        let season = u32::from_str(raw)?;
        urls = download_season(&s, season, &language)?;
    } else {
        urls = download_series(&s, &language)?;
    }

    println!("{}", "Downloading episodes...".yellow());
    for url in urls {
        println!(
            "Downloading: Season: {}, Episode: {} from: {:?}...",
            url.episode.season.id,
            url.episode.id + 1,
            url.host
        );
        let absolute_output: String;
        if generate_name {
            absolute_output = format!(
                "{}/{} S{}E{} - {}.%(ext)s",
                &output,
                url.episode.season.get_name(),
                url.episode.season.id,
                url.episode.id + 1, // starts at 0
                url.episode.get_name(&language).replace("/", "")
            );
        } else {
            absolute_output = format!("{}/%(title)s.%(ext)s", &output);
        }
        if use_youtube_dl {
            youtube_dl(url.streamer_url.as_str(), absolute_output.as_str())?;
            continue;
        }
        let downloader = match url.host {
            Host::Vivo => Some(vivo::new(url.streamer_url.as_str())),
            Host::Vidoza => Some(vidoza::new(url.streamer_url.as_str())),
            _ => None,
        };
        if downloader.is_none() {
            println!(
                "{}",
                "Unable to download video. Using youtube-dl instead.".yellow()
            );
            youtube_dl(url.streamer_url.as_str(), absolute_output.as_str())?;
            continue;
        }
        let downloader = downloader.unwrap()?;
        let mut absolute_output = absolute_output
            .replace("%(title)s", downloader.get_name().as_str())
            .replace("%(ext)s", downloader.get_extension().as_str());
        downloader.download_to_file(&mut File::create(&absolute_output)?)?;
    }
    println!("Everything should be saved in: {}/\nEnjoy!", output);
    Ok(())
}

fn youtube_dl(url: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let mut p = Command::new("youtube-dl");
    p.arg(url).arg("--output").arg(output).output()?;
    Ok(())
}

pub fn random_string(n: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric).take(n).collect()
}

fn generate_account(amount: u32) -> Result<(), Box<dyn Error>> {
    if amount == 0 {
        return Ok(());
    }
    let acc = Account::create(random_string(16), Email::new_random()?, random_string(8));
    if acc.is_err() {
        generate_account(amount)?;
        return Ok(());
    }
    let acc = acc?;
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("accounts.txt")?;
    write!(
        file,
        "{}",
        format!("\n{}:{}", acc.email.to_string(), acc.password)
    )?;
    generate_account(amount - 1)?;
    Ok(())
}

fn download_series(s: &Series, language: &Language) -> Result<Vec<Url>, Box<dyn Error>> {
    let mut urls: Vec<Url> = Vec::new();
    let series_len = s.get_season_count();
    for season in 1..series_len {
        let vec = download_season(&s, season, &language)?;
        for vec_entry in vec {
            urls.push(vec_entry);
        }
    }
    Ok(urls)
}

fn download_season(
    s: &Series,
    season: u32,
    language: &Language,
) -> Result<Vec<Url>, Box<dyn Error>> {
    let mut urls: Vec<Url> = Vec::new();
    let season_len = s.get_season(season).unwrap().get_episode_count();
    for episode in 0..season_len {
        let url = download_episode(&s, season, episode as u32, &language);
        if url.is_ok() {
            urls.push(url?);
        }
    }
    Ok(urls)
}

fn download_episode(
    s: &Series,
    season: u32,
    episode: u32,
    language: &Language,
) -> Result<Url, Box<dyn Error>> {
    let list_raw = read_to_string("accounts.txt")?;
    let mut list: Vec<&str> = list_raw.split("\n").collect();
    list.shuffle(&mut rand::thread_rng());
    let acc = Account::from_str(list[0]);
    if acc.is_err() {
        // looks like whatever whe got wasn't an account. Retry.
        return download_episode(s, season, episode, &language);
    }
    let acc = acc?;
    let url = s
        .get_season(season)?
        .get_episode(episode)?
        .get_stream_url(&language);
    if url.is_err() {
        return Err(url.err().unwrap())?;
    }
    let url = url?.get_site_url(&acc);
    if url.is_err() {
        return download_episode(&s, season, episode, &language);
    }
    Ok(url?)
}
