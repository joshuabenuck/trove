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
    #[serde(rename = "currentTime|datetime")]
    pub current_time: String,
    #[serde(rename = "nextAdditionTime|datetime")]
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
    // pub display_item_data: Value,
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
        self.standard_products
            .sort_by_key(|p| p.human_name.to_lowercase());
    }

    fn images(&self) -> Vec<&str> {
        self.standard_products
            .iter()
            .map(|product| product.image.as_str())
            .collect()
    }
}

trait TroveCache {
    fn chunk_url(&self, i: usize) -> String;
    fn trove_url(&self) -> &'static str;
    fn feed_doc(&self) -> Result<Value, Error>;
    fn chunks(&self, root: &Value) -> usize;
    fn get_trove_feed(&self) -> Result<Value, Error>;
    fn invalidate(&self) -> Result<(), Error>;
}

impl TroveCache for Cache {
    fn chunk_url(&self, i: usize) -> String {
        format!(
            "https://www.humblebundle.com/api/v1/trove/chunk?property=start&direction=desc&index={}",
            i
        )
    }

    fn trove_url(&self) -> &'static str {
        "https://www.humblebundle.com/subscription/trove"
    }

    fn feed_doc(&self) -> Result<Value, Error> {
        let text = self.retrieve(self.trove_url())?;
        let doc = Document::from(str::from_utf8(&text)?);
        let data = doc
            .find(Attr("id", "webpack-monthly-trove-data"))
            .next()
            .unwrap()
            .text();
        let root: Value = serde_json::from_str(data.as_str())?;
        Ok(root)
    }

    fn chunks(&self, root: &Value) -> usize {
        debug!("Extracting number of chunks");
        let chunks: usize = match &root["chunks"] {
            Value::Number(number) => {
                number.as_u64().expect("Unable to convert chunks to u64") as usize
            }
            _ => panic!("Unable to get chunks value!"),
        };
        chunks
    }

    fn get_trove_feed(&self) -> Result<Value, Error> {
        let mut root = self.feed_doc()?;
        let chunks = self.chunks(&root);
        debug!("Getting product list");
        let mut products = Vec::new();
        // match root
        //     .get_mut("standardProducts")
        //     .expect("Unable to get product list")
        // {
        //     Value::Array(array) => array,
        //     _ => panic!("Unexpected value in standard_products field"),
        // };
        for i in 0..chunks {
            let bytes = self.retrieve(self.chunk_url(i).as_str())?;
            let chunk: Vec<Value> = serde_json::from_str(str::from_utf8(&bytes)?)?;
            products.extend(chunk);
        }
        root.as_object_mut()
            .expect("Unable to get root")
            .insert("standardProducts".to_string(), Value::Array(products));
        Ok(root)
    }

    fn invalidate(&self) -> Result<(), Error> {
        // This is a bit weird. We retrieve the cached value only to determine
        // how many chunk urls we need to invalidate. This is needed because we
        // do not save the extracted chunk value in our exports.
        let root = self.feed_doc()?;
        let chunks = self.chunks(&root);
        self.invalidate(self.trove_url())?;
        for i in 0..chunks {
            self.invalidate(self.chunk_url(i).as_str())?;
        }
        Ok(())
    }
}

pub struct TroveFeed {
    cache: Cache,
    json: String,
    feed: Feed,
}

impl TroveFeed {
    pub fn new(cache: Cache, dir: &PathBuf) -> Result<TroveFeed, Error> {
        let root = cache.get_trove_feed()?;
        let json = serde_json::to_string_pretty(&root)?;
        let mut trove_feed = TroveFeed {
            cache,
            json,
            feed: serde_json::from_value(root)?,
        };
        if trove_feed.expired() {
            eprintln!("Refreshing expired cache.");
            TroveCache::invalidate(&trove_feed.cache)?;
            return TroveFeed::new(trove_feed.cache, dir);
        }
        let mut products: Vec<String> = Vec::new();
        // Dedup the list
        trove_feed.feed.standard_products.retain(|p| {
            if !products.contains(&p.machine_name) {
                products.push(p.machine_name.clone());
                return true;
            }
            return false;
        });
        let newly_added = &trove_feed.feed.newly_added;
        let standard_products = &mut trove_feed.feed.standard_products;
        newly_added.iter().for_each(|p| {
            if !products.contains(&p.machine_name) {
                products.push(p.machine_name.clone());
                standard_products.push(p.clone());
            }
        });
        trove_feed.feed.alphabetically();
        trove_feed.save(&dir.join("trove_feed.json"))?;
        trove_feed.backup(dir)?;
        Ok(trove_feed)
    }

    pub fn expired(&self) -> bool {
        let expiration = NaiveDateTime::parse_from_str(
            &self.feed.countdown_timer_options.next_addition_time,
            "%Y-%m-%dT%H:%M:%S%.f",
        )
        .expect("Error parsing nextAdditionTime");
        debug!("Expiration: {}", expiration);
        if Utc::now().timestamp() > expiration.timestamp() {
            return true;
        }
        return false;
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
        let newly_added = &feed.newly_added;
        let standard_products = &mut feed.standard_products;
        newly_added.iter().for_each(|p| {
            if !products.contains(&p.machine_name) {
                products.push(p.machine_name.clone());
                standard_products.push(p.clone());
            }
        });
        feed.alphabetically();
        Ok(TroveFeed { cache, json, feed })
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), Error> {
        let mut file = File::create(path)?;
        file.write(self.json.as_bytes())?;
        Ok(())
    }

    pub fn backup(&self, dir: &PathBuf) -> Result<(), Error> {
        let filename = Utc::now().format("trove_feed-%Y-%m-%d.json").to_string();
        info!("Creating backup: {}.", &filename);
        self.save(&dir.join(filename))?;
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

    pub fn sort_newest_to_oldest(&mut self) {
        self.feed.newest_to_oldest();
    }

    pub fn sort_alphabetically(&mut self) {
        self.feed.alphabetically();
    }
}
