/// This module handles the deserialization of the humble bundle monthly trove metadata feed.
/// It provides operations that deal with the contents of the feed itself.
use crate::cache::Cache;
use chrono::{NaiveDateTime, Utc};
use failure::Error;
use log::{debug, info, warn};
use select::{document::Document, predicate::Attr};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerOptions {
    pub current_time: String,
    pub next_addition_time: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Url {
    pub web: String,
    pub bittorrent: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Download {
    //pub uploaded_at: Option<String>,
    pub machine_name: String,
    pub name: String,
    pub url: Url,
    pub file_size: u64,
    //pub small: Option<u8>,
    pub md5: String,
    //pub sha1: Option<String>,
    pub size: Option<String>,
    //pub timestamp: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct CarouselContent {
    pub youtube_link: Option<Vec<String>>,
    pub thumbnail: Vec<String>,
    pub screenshot: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Publisher {
    pub publisher_name: String,
    pub publisher_uri: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Developer {
    pub developer_name: String,
    pub developer_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Product {
    pub all_access: bool,
    pub background_image: Option<String>, // can be null
    pub background_color: Option<String>, // can be null
    pub carousel_content: CarouselContent,
    pub date_added: u32,
    pub description_text: String,
    pub developers: Option<Vec<Developer>>,
    pub downloads: HashMap<String, Download>,
    pub human_name: String,
    pub humble_original: Option<bool>, // can be null
    pub image: String,
    pub logo: Option<String>,
    #[serde(rename = "machine_name")]
    pub machine_name: String,
    pub marketing_blurb: Value, //Map {text, style} or String,
    pub popularity: u16,
    pub publishers: Value,                  // can be null Vec<Publisher>,
    pub trove_showcase_css: Option<String>, // can be null
    pub youtube_link: Option<String>,       // can be null
}

trait ProductVec {
    fn contains(&self, machine_name: &str) -> bool;
}

impl ProductVec for Vec<Product> {
    fn contains(&self, human_name: &str) -> bool {
        for product in self.iter() {
            if product.human_name == human_name {
                return true;
            }
        }
        return false;
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    pub all_access: Vec<String>,
    pub download_platform_order: Vec<String>,
    pub newly_added: Vec<Product>,
    pub display_item_data: Value,
    pub countdown_timer_options: TimerOptions,
    pub standard_products: Vec<Product>,
    //pub chunks: u8,
    //pub games_per_chunk: u8,
}

impl Feed {
    fn newest_to_oldest(&mut self) {
        self.standard_products.sort_by_key(|p| p.date_added);
        self.standard_products.reverse();
    }

    fn alphabetically(&mut self) {
        self.standard_products.sort_by_key(|p| p.human_name.clone());
    }

    fn images(&self) -> Vec<&str> {
        self.standard_products
            .iter()
            .map(|product| product.image.as_str())
            .collect()
    }
}

pub struct TroveFeed {
    cache: Cache,
    json: String,
    feed: Feed,
}

fn chunk_url(i: usize) -> String {
    format!(
        "https://www.humblebundle.com/api/v1/trove/chunk?index={}",
        i
    )
}

impl TroveFeed {
    fn retrieve(cache: &Cache) -> Result<Value, Error> {
        const TROVE_URL: &str = "https://www.humblebundle.com/monthly/trove";
        let text = cache.retrieve(&TROVE_URL)?;
        let doc = Document::from(str::from_utf8(&text)?);
        let data = doc
            .find(Attr("id", "webpack-monthly-trove-data"))
            .next()
            .unwrap()
            .text();
        let mut root: Value = serde_json::from_str(data.as_str())?;
        debug!("Extracting number of chunks");
        let chunks: usize = match &root["chunks"] {
            Value::Number(number) => {
                number.as_u64().expect("Unable to convert chunks to u64") as usize
            }
            _ => panic!("Unable to get chunks value!"),
        };
        let expiration = match &root["countdownTimerOptions"]["nextAdditionTime"] {
            Value::String(string) => {
                NaiveDateTime::parse_from_str(string.as_str(), "%Y-%m-%dT%H:%M:%S%.f")
                    .expect("Error parsing nextAdditionTime")
            }
            _ => panic!("Unable to get nextAdditionTime!"),
        };
        debug!("Expiration: {}", expiration);
        if Utc::now().timestamp() > expiration.timestamp() {
            println!("Resetting cache.");
            info!("Resetting cache.");
            cache.invalidate(TROVE_URL)?;
            for i in 0..chunks {
                cache.invalidate(chunk_url(i).as_str())?;
            }
            return TroveFeed::retrieve(cache);
        }
        debug!("Getting product list");
        let products = match root
            .get_mut("standardProducts")
            .expect("Unable to get product list")
        {
            Value::Array(array) => array,
            _ => panic!("Unexpected value in standard_products field"),
        };
        for i in 0..chunks {
            let bytes = cache.retrieve(chunk_url(i).as_str())?;
            let chunk: Vec<Value> = serde_json::from_str(str::from_utf8(&bytes)?)?;
            products.extend(chunk);
        }
        Ok(root)
    }

    pub fn new(cache: Cache) -> Result<TroveFeed, Error> {
        let root: Value = TroveFeed::retrieve(&cache)?;
        let json = serde_json::to_string_pretty(&root)?;
        let mut trove_feed = TroveFeed {
            cache,
            json,
            feed: serde_json::from_value(root)?,
        };
        trove_feed.feed.alphabetically();
        let mut products: Vec<String> = Vec::new();
        trove_feed.feed.standard_products.retain(|p| {
            if !products.contains(&p.machine_name) {
                products.push(p.machine_name.clone());
                return true;
            }
            return false;
        });
        Ok(trove_feed)
    }

    pub fn cache_images(&self) {
        self.feed.images().iter().for_each(|image| {
            if let Err(err) = self.cache.retrieve(image) {
                warn!("{}", err);
            }
        });
        self.cache_screenshots();
        self.cache_thumbnails();
    }

    pub fn cache_thumbnails(&self) {
        (&self.feed.standard_products)
            .iter()
            .flat_map(|p| &p.carousel_content.thumbnail)
            .for_each(|url| {
                if let Err(err) = self.cache.retrieve(url.as_str()) {
                    warn!("{}", err);
                }
            });
    }

    pub fn cache_screenshots(&self) {
        (&self.feed.standard_products)
            .iter()
            .flat_map(|p| &p.carousel_content.screenshot)
            .for_each(|url| {
                if let Err(err) = self.cache.retrieve(url.as_str()) {
                    warn!("{}", err);
                }
            });
    }

    pub fn load(cache: Cache, path: &PathBuf) -> Result<TroveFeed, Error> {
        let mut json = String::new();
        let mut file = File::open(path)?;
        file.read_to_string(&mut json)?;
        let mut feed: Feed = serde_json::from_str(&json)?;
        let mut products: Vec<String> = Vec::new();
        feed.standard_products.retain(|p| {
            if !products.contains(&p.machine_name) {
                products.push(p.machine_name.clone());
                return true;
            }
            return false;
        });
        feed.alphabetically();
        Ok(TroveFeed { cache, json, feed })
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), Error> {
        let mut file = File::create(path)?;
        file.write(self.json.as_bytes())?;
        Ok(())
    }

    pub fn backup(&self, _dir: &PathBuf) -> Result<(), Error> {
        Ok(())
    }

    pub fn diff(&self, older: TroveFeed) {
        let mut new_names = Vec::new();
        for product in &self.feed.standard_products {
            if !older.feed.standard_products.contains(&product.human_name) {
                new_names.push(product.human_name.clone());
            }
        }

        let mut old_names = Vec::new();
        for product in older.feed.standard_products {
            if !&self.feed.standard_products.contains(&product.human_name) {
                old_names.push(product.human_name.clone());
            }
        }

        println!("Added titles:");
        println!("-------------");
        new_names.iter().for_each(|name| println!("{}", name));
        println!("");
        println!("Deleted titles:");
        println!("---------------");
        old_names.iter().for_each(|name| println!("{}", name));
    }

    pub fn products(&self) -> &Vec<Product> {
        &self.feed.standard_products
    }
}
