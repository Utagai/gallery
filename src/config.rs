use std::env;
use std::fs::File;
use std::path::PathBuf;

use anyhow::{Error, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GalleryConfig {
    dirs: Vec<PathBuf>,
}

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
