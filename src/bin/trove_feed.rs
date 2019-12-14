/*
trove_feed
Phase 1: Preservation
X Download Trove feed as json (name with date, copy to trove.json)
X Allow comparison with previous copies (by filename)
X Keep backups of previous feeds
Download all images

trove
Phase 2: Downloads
X Create processed trove database
Merge in data from newer feeds
X Detect which games are downloaded
X Look for items still in Downloads folder
X Move to trove
Separate installers from installed Trove games
Detect whether download needs to be installed or not
Detect which games are installed
Launch installers
Launch games

doorways
Phase 3: Doorways
Integrate with Doorways launcher
*/
extern crate trove;

use clap::{App, Arg};
use dirs;
use env_logger;
use failure::Error;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use trove::{Cache, TroveFeed};

fn run() -> Result<(), Error> {
    env_logger::init();
    let matches = App::new("trove_feed")
        .about("A utility to manage Humble Bundle Trove feed data")
        .arg(
            Arg::with_name("list")
                .long("list")
                .help("List the titles in the trove"),
        )
        .arg(
            Arg::with_name("newest")
                .long("newest")
                .help("Sort list newest to oldest"),
        )
        .arg(
            Arg::with_name("diff")
                .long("diff")
                .takes_value(true)
                .help("Diff the titles in the current set with the ones in the specified backup"),
        )
        .arg(
            Arg::with_name("new")
                .long("new")
                .help("Display the newly added titles"),
        )
        .arg(
            Arg::with_name("update")
                .long("update")
                .help("Update trove_feed.json"),
        )
        .arg(
            Arg::with_name("cache-images")
                .long("cache-images")
                .help("Cache the images referenced in the Trove feed"),
        )
        .get_matches();
    let trove_dir: PathBuf = dirs::home_dir()
        .expect("Unable to find home directory!")
        .join(".trove");
    let trove_json = trove_dir.join("trove_feed.json");
    if !trove_dir.exists() {
        fs::create_dir_all(&trove_dir)?;
    }
    let cache_dir = &trove_dir.join("cache");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }
    let cache = Cache::new(cache_dir);
    let mut feed = if !trove_json.exists() || matches.is_present("update") {
        TroveFeed::new(cache, &trove_dir)?
    } else {
        TroveFeed::load(cache, &trove_json)?
    };
    if feed.expired() {
        eprintln!("Warning: Feed is expired. Run --update to correct.");
    }
    if matches.is_present("list") {
        if matches.is_present("newest") {
            feed.sort_newest_to_oldest();
        }
        feed.products()
            .iter()
            .for_each(|p| println!("{}", p.human_name));
    }
    if matches.is_present("cache-images") {
        feed.cache_images();
    }
    if let Some(to_diff) = matches.value_of("diff") {
        let cache = Cache::new(cache_dir);
        println!("Loading old version.");
        let old = TroveFeed::load(cache, &to_diff.into())?;
        println!("Diffing");
        feed.diff(old);
    }
    Ok(())
}

fn main() {
    match run() {
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
        Ok(_) => (),
    }
}
