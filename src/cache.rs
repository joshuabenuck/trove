/// This module provides a local cache of web URLs. It is intended to be the equivalent of
/// a browser's cache. It currently doesn't expire any entries in the cache.
///
/// TODO:
/// - Allow for a forced overwrite of a cache entry
/// - Enable preservation of old copies of cache entries
/// - Provide a means to return old cached copies of entries
/// - Use this capability to backup old copies of the humble bundle monthly feed
extern crate sha2;
extern crate log;

use std::path::PathBuf;
use sha2::Digest;
use std::fs;
use std::io::{Read, Error};
use log::{debug, error};

fn sha256(url: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.input(url.as_bytes());
    hex::encode(&hasher.result())
}

pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new<T: Into<PathBuf>>(root: T) -> Cache {
        let cache = Cache { root: root.into() };
        if !cache.root.exists() {
            debug!("creating: {}", cache.root.display());
            if let Err(result) = fs::create_dir_all(&cache.root) {
                error!("error: {:?}", result);
            }
        }
        return cache;
    }

    pub fn retrieve(&self, url: &str) -> Result<Vec<u8>, Error> {
        let hash = sha256(url);
        let cached = self.root.join(&hash);
        debug!("{:?}", hash);
        if !cached.exists() {
            // TODO: Add cache expiration
            debug!("caching: {}", url);
            let mut resp = reqwest::get(url).unwrap();
            assert!(resp.status().is_success());
            let mut buffer = Vec::new();
            resp.read_to_end(&mut buffer)?;
            fs::write(&cached, buffer)?;
            fs::write(self.root.join(format!("{}.url", &hash)), url)?;
        }
        Ok(fs::read(cached)?)
    }

    pub fn get_versions(&self, _url: &str) -> Vec<String> {
        Vec::<String>::new()
    }

    pub fn retrieve_version(&self, _index: u32) {}

    pub fn force_retrieve(&self, url: &str) -> Result<Vec<u8>, Error> {
        let hash = sha256(url);
        let cached = self.root.join(&hash);
        if cached.exists() {
            fs::remove_file(cached)?;
        }
        self.retrieve(url)
    }
}
