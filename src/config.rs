use std::env;
use std::fs::{read_dir, DirEntry, File, ReadDir};
use std::path::PathBuf;
use std::str;

use anyhow::{Context, Error, Result};
use serde::{Deserialize, Serialize};

pub fn load_config(config_path: &str) -> Result<GalleryConfig> {
    let config_file = File::open(config_path)?;
    let gallery_cfg = serde_json::from_reader(config_file)?;
    Ok(gallery_cfg)
}

// May exit.
pub fn parse_config_path_from_args_or_die() -> Result<String> {
    if let Some(first_arg) = env::args().nth(1) {
        Ok(first_arg)
    } else {
        Err(Error::msg("no configuration file argument specified"))
    }
}

#[derive(Deserialize, Debug)]
pub struct GalleryConfig {
    dirs: Vec<PathBuf>,
}

#[derive(Serialize, Debug)]
pub struct Image {
    path: PathBuf,
    bytes: String,
}

impl Image {
    fn new(path: PathBuf) -> Result<Image> {
        // TODO: I think we are doing 3 passes here to get to a base64 string:
        // Read pass
        // Write pass (happens in lockstep with above, I assume, due to io::copy)
        // Byte -> String pass
        // I think we can get away with two:
        // Read pass (into bytes)
        // base64::encode() (I assume only does one pass)
        let mut image_file = File::open(&path)?;
        let mut enc = base64::write::EncoderWriter::new(Vec::new(), base64::STANDARD);
        std::io::copy(&mut image_file, &mut enc)?;
        Ok(Image {
            path,
            bytes: str::from_utf8(&enc.finish()?)?.to_string(),
        })
    }
}

#[derive(Serialize, Debug)]
pub struct Gallery {
    pub images: Vec<Image>,
}

impl Gallery {
    pub fn new(cfg: &GalleryConfig) -> Result<Gallery> {
        let mut dir_iters: Vec<ReadDir> = Vec::with_capacity(cfg.dirs.len());
        for dir in &cfg.dirs {
            let path = dir.as_path().display().to_string();
            let context_msg = format!("failed to open directory '{}'", &path);
            let dir_iter = read_dir(&path).context(context_msg)?;
            dir_iters.push(dir_iter);
        }

        let results: Vec<DirEntry> = dir_iters
            .iter_mut()
            .flatten()
            .collect::<Result<Vec<DirEntry>, std::io::Error>>()?;

        let paths = results.iter().map(|x| x.path()).collect::<Vec<PathBuf>>();
        let mut images: Vec<Image> = Vec::with_capacity(paths.len());
        for path in paths {
            images.push(Image::new(path)?)
        }

        Ok(Gallery { images })
    }
}
