use std::fs::File;
use std::io::{Error, Read, Write};
use std::path::PathBuf;
use url::{ParseError, Url};

pub fn create_file(name: PathBuf, contents: &str) -> Result<(), Error> {
    println!("Creating file: {}", name.display());
    File::create(name)?.write(contents.as_bytes())?;
    Ok(())
}

pub fn copy_to_file(name: PathBuf, buffer: &Vec<u8>) -> Result<(), Error> {
    println!("Creating file: {}", name.display());
    File::create(name)?.write(buffer)?;
    Ok(())
}

pub fn read_file(name: PathBuf) -> Result<Vec<u8>, Error> {
    let mut buffer = Vec::new();
    File::open(name)?.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn url_path(url: &str) -> Result<String, ParseError> {
    Ok(Url::parse(url)?.path().to_string().clone())
}

pub fn url_path_ext(url: String) -> Option<String> {
    match url_path(&url) {
        Ok(path) => extension(&path),
        Err(_) => None,
    }
}

pub fn extension(name: &str) -> Option<String> {
    let mut iter = name.rsplit(".");
    Some(iter.next()?.split("?").next()?.to_string())
    /*match PathBuf::from(name).extension() {
        Some(ext) => {
            match ext.to_os_string().into_string() {
                Ok(ext) => Some(ext),
                Err(ext) => None,
            }
        },
        None => None,
    }*/
}
