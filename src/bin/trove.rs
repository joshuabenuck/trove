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
        .get_matches();
    let trove_dir = dirs::home_dir()
        .expect("Unable to find home directory!")
        .join(".trove");
    let trove_games_json = trove_dir.join("trove_games.json");
    let games = if trove_games_json.exists() {
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
    if matches.is_present("list") {
        for game in &games.games {
            println!("{}", game.human_name);
        }
    }
    println!("Game count: {}", games.games.len());
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
