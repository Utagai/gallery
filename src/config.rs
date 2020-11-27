use std::env;
use std::fs::{read_dir, DirEntry, File, ReadDir};
use std::path::PathBuf;

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
pub struct Gallery {
    pub dir_entries: Vec<PathBuf>,
}

impl Gallery {
    pub fn new(cfg: &GalleryConfig) -> Result<Gallery> {
        let mut dir_iters: Vec<ReadDir> = Vec::new();
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

        Ok(Gallery {
            dir_entries: results.iter().map(|x| x.path()).collect(),
        })
    }
}
