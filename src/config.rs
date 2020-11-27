use std::env;
use std::fs::{read_dir, DirEntry, File, ReadDir};
use std::path::PathBuf;

use anyhow::{Error, Result};
use serde::Deserialize;

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

#[derive(Debug)]
pub struct Gallery {
    pub dir_entries: Vec<PathBuf>,
}

impl Gallery {
    pub fn new(cfg: &GalleryConfig) -> Result<Gallery> {
        let mut dir_iters: Vec<ReadDir> = Vec::new();
        for dir in &cfg.dirs {
            let dir_iter = read_dir(dir.as_path())?;
            dir_iters.push(dir_iter);
        }

        let results: Vec<DirEntry> = dir_iters
            .iter_mut()
            .flatten()
            .collect::<Result<Vec<DirEntry>, std::io::Error>>()?;

        Ok(Gallery {
            dir_entries: results.iter().map(|x| x.path()).collect(),
        })
    }
}
