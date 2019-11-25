use crate::cache::Cache;
use crate::trove_feed::{Product, TroveFeed};
use crate::util::{extension, url_path_ext};
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct TroveGame {
    pub machine_name: String,
    pub human_name: String,
    pub description: String,
    pub date_added: u32,
    pub downloaded: bool, // eventually HashMap
    pub installed: bool,
    pub executable: PathBuf,
    pub download_urls: HashMap<String, String>,
    pub downloads: HashMap<String, PathBuf>,
    pub logo: Option<String>,
    pub image: String,
    pub screenshots: Vec<String>,
    pub thumbnails: Vec<String>,
    pub trailer: Option<String>,
    pub last_seen_on: String,
    pub removed_from_trove: bool,
}

/*
 * trait Into<T>: Sized {fn into(self) -> T;}
 * trait From<T>: Sized {fn from(T) -> Self;}
 */
impl From<&Product> for TroveGame {
    fn from(p: &Product) -> TroveGame {
        let mut download_urls = HashMap::<String, String>::new();
        download_urls.insert(
            "windows".to_string(),
            p.downloads["windows"].url.web.clone(),
        );
        TroveGame {
            machine_name: p.machine_name.clone(),
            human_name: p.human_name.clone(),
            description: p.description_text.clone(),
            date_added: p.date_added,
            downloaded: false,
            installed: false,
            executable: "".to_string().into(),
            downloads: download_urls
                .iter()
                .map(|(o, u)| {
                    (
                        o.clone(),
                        PathBuf::from(PathBuf::from(u).file_name().unwrap()).clone(),
                    )
                })
                .collect(),
            download_urls: download_urls,
            logo: p.logo.clone(),
            image: p.image.clone(),
            screenshots: p.carousel_content.screenshot.clone(),
            thumbnails: p.carousel_content.thumbnail.clone(),
            trailer: p.youtube_link.clone(),
            last_seen_on: "".to_string(),
            removed_from_trove: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Trove {
    pub downloads: PathBuf,
    pub root: PathBuf,
    pub number_downloaded: u32,
    pub total: u32,
    pub games: Vec<TroveGame>,
    //pub installed_games: Vec<String>,
    //pub downloaded_games: Vec<String>,
    //pub not_downloaded_games: Vec<String>,
}

impl Trove {
    pub fn load(dir: &PathBuf) -> Result<Trove, Error> {
        let file = fs::File::open(dir.join("trove_games.json"))?;
        let trove: Trove = serde_json::from_reader(file)?;
        Ok(trove)
    }

    pub fn from(root: &PathBuf, downloads: &PathBuf, feed: TroveFeed) -> Result<Trove, Error> {
        let mut trove = Trove::new(root, downloads)?;
        trove.add_games(feed);
        Ok(trove)
    }

    pub fn new(root: &PathBuf, downloads: &PathBuf) -> Result<Trove, Error> {
        let trove = Trove {
            downloads: downloads.clone(),
            root: root.clone(),
            number_downloaded: 0,
            total: 0,
            games: Vec::new(),
        };
        assert!(trove.root.exists());
        assert!(trove.downloads.exists());
        Ok(trove)
    }

    pub fn add_games(&mut self, feed: TroveFeed) {
        for product in feed.products() {
            self.games.push(product.into());
        }
    }

    pub fn update_download_status(&mut self) {
        let mut count = 0;
        for game in self.games.iter_mut() {
            let installer = game.downloads["windows"].to_str().unwrap();
            game.downloaded = self.root.join(installer).exists();
            if game.downloaded {
                count += 1;
            }
        }
        self.number_downloaded = count;
        self.total = self.games.len() as u32;
        println!(
            "Downloaded: {}; Total: {}",
            &self.number_downloaded, &self.total
        );
    }

    pub fn downloaded(&self) -> Vec<&TroveGame> {
        (&self.games).iter().filter(|g| g.downloaded).collect()
    }

    pub fn not_downloaded(&self) -> Vec<&TroveGame> {
        (&self.games).iter().filter(|g| !g.downloaded).collect()
    }

    /// Save current trove game metadata to disk
    /// Pull down copies of all game related images
    /// TODO: Throttle or rate limit this method
    pub fn cache_all_metadata(&self) -> Result<(), Error> {
        /*let metadata_root = self.root.join("metadata/");
        assert!(metadata_root.exists());
        for (name, game) in self.games.iter() {
            match url_path_ext(game.image.clone()) {
                None => println!("{} has no extension.", &game.image),
                Some(ext) => {
                    println!("{} is the ext for {}", ext, &game.image);
                    let image_filename = metadata_root.join(format!("{}.{}", name, ext));
                    fs::write(image_filename, self.cache.retrieve(&game.image)?)?;
                }
            }
            if let Some(logo) = &game.logo {
                match url_path_ext(logo.clone()) {
                    None => println!("{} has no extension.", &logo),
                    Some(ext) => {
                        println!("{} is the ext for {}", ext, &logo);
                        let image_filename = metadata_root.join(format!("{}_logo.{}", name, ext));
                        fs::write(image_filename, self.cache.retrieve(&logo)?)?;
                    }
                }
            }
            for (index, url) in game.thumbnails.iter().enumerate() {
                let img_format = match extension(url) {
                    Some(ext) => ext,
                    None => panic!("image doesn't have an extension"),
                };
                let target = format!("{}_t{}.{}", name, index, img_format);
                fs::write(metadata_root.join(target), cache.retrieve(url)?)?;
            }
            for (index, url) in game.screenshots.iter().enumerate() {
                let img_format = match extension(url) {
                    Some(ext) => ext,
                    None => panic!("image doesn't have an extension"),
                };
                let target = format!("{}_s{}.{}", name, index, img_format);
                fs::write(metadata_root.join(target), cache.retrieve(url)?)?;
            }
        }*/
        Ok(())
    }

    pub fn format(&self, g: &TroveGame) -> String {
        format!("{} {} {}", g.date_added, g.human_name, g.downloaded)
    }

    pub fn stray_downloads(&self) -> Vec<PathBuf> {
        let downloads = Path::new(&self.downloads);
        assert!(downloads.exists());
        (&self.games)
            .iter()
            .filter_map(|game| {
                let installer = Path::new(&game.downloads["windows"])
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                let full_installer_path = downloads.join(&installer);
                match full_installer_path.exists() {
                    true => Some(full_installer_path),
                    false => None,
                }
            })
            .collect()
    }

    pub fn move_downloads(&self) -> Vec<PathBuf> {
        self.stray_downloads()
            .iter()
            .filter_map(|download| {
                let dest = self.root.join(download.file_name().unwrap());
                println!(
                    "Moving {} to {}.",
                    download.to_str().unwrap(),
                    dest.to_str().unwrap()
                );
                if dest.exists() {
                    warn!("exists, skipping: {}", dest.to_str().unwrap());
                    return Some(download);
                }
                let result = fs::copy(download, &dest);
                match result {
                    Err(e) => {
                        warn!("{}: {}", e, dest.to_str().unwrap());
                        Some(download)
                    }
                    Ok(_) => {
                        let result = fs::remove_file(download);
                        match result {
                            Err(e) => {
                                warn!("{}: removing {}", e, download.to_str().unwrap());
                                Some(download)
                            }
                            Ok(_) => None,
                        }
                    }
                }
            })
            .cloned()
            .collect()
    }
}
