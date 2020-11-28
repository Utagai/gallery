use std::fs::{read_dir, DirEntry, ReadDir};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::config;

#[derive(Serialize, Debug)]
pub struct Gallery {
    pub paths: Vec<PathBuf>,
}

impl Gallery {
    pub fn new(cfg: &config::GalleryConfig) -> Result<Gallery> {
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

        Ok(Gallery { paths })
    }

    pub fn has(&self, path: &Path) -> bool {
        self.paths.iter().any(|p| p == path)
    }
}
