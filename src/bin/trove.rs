extern crate trove;

use clap::{App, Arg};
use failure::Error;
use std::path::PathBuf;
use std::process::exit;
use trove::{Cache, Trove, TroveFeed};

fn run() -> Result<(), Error> {
    let matches = App::new("trove")
        .about("Utility to manage games from the Humble Bundle Trove")
        .arg(
            Arg::with_name("list")
                .long("list")
                .help("List the games in the trove"),
        )
        .arg(
            Arg::with_name("stray-downloads")
                .long("stray-downloads")
                .help("Show all trove downloads still in the download directory"),
        )
        .arg(
            Arg::with_name("move-downloads")
                .long("move-downloads")
                .help("Move all stray downloads to the trove"),
        )
        .arg(
            Arg::with_name("downloads")
                .long("downloads")
                .takes_value(true)
                .help("Directory to use to look for downloads"),
        )
        .arg(
            Arg::with_name("root")
                .long("root")
                .takes_value(true)
                .help("Directory to use as the root of the local Trove cache"),
        )
        .arg(
            Arg::with_name("downloaded")
                .long("downloaded")
                .takes_value(true)
                .default_value("true")
                .help("Filter games by whether they are downloaded"),
        )
        .get_matches();
    let trove_dir = dirs::home_dir()
        .expect("Unable to find home directory!")
        .join(".trove");
    let trove_games_json = trove_dir.join("trove_games.json");
    let mut trove = if trove_games_json.exists() {
        Trove::load(&trove_games_json)?
    } else {
        if !matches.is_present("downloads") || !matches.is_present("root") {
            eprintln!("Must pass in both --downloads and --root when creating the cache.");
            exit(1);
        }
        let downloads: PathBuf = matches.value_of("downloads").unwrap().into();
        let root: PathBuf = matches.value_of("root").unwrap().into();
        let mut trove = Trove::new(&root, &downloads)?;
        let cache = Cache::new(trove_dir.join("cache"));
        let trove_feed = TroveFeed::load(cache, &trove_dir.join("trove.json"))?;
        trove.add_games(trove_feed);
        trove
    };
    if matches.is_present("stray-downloads") {
        for download in trove.stray_downloads() {
            println!("{}", download.display());
        }
    }
    if matches.is_present("move-downloads") {
        trove.move_downloads();
    }
    trove.update_download_status();
    let mut games = trove.games.iter().map(|g| g).collect();
    if matches.is_present("downloaded") {
        let downloaded = matches.value_of("downloaded").unwrap().parse::<bool>()?;
        if downloaded {
            games = trove.downloaded();
        } else {
            games = trove.not_downloaded();
        }
    }
    if matches.is_present("list") {
        for game in &games {
            println!("{}", game.human_name);
        }
    }
    println!("Game count: {}", games.len());
    Ok(())
}

fn main() {
    match run() {
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
        Ok(_) => (),
    }
}
