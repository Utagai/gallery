use std::fs::{read_dir, DirEntry, ReadDir};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::config;

#[derive(Serialize, Debug)]
pub struct Gallery {
    pub paths: Vec<PathBuf>,
    #[serde(skip_serializing)]
    stop: Arc<AtomicBool>,
    #[serde(skip_serializing)]
    inotify_thread: Option<JoinHandle<()>>,
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

        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let inotify_thread = Some(thread::spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                println!("Hello world!");
                thread::sleep(Duration::from_millis(1000));
            }
        }));

        Ok(Gallery {
            paths,
            stop,
            inotify_thread,
        })
    }

    pub fn has(&self, path: &Path) -> bool {
        self.paths.iter().any(|p| p == path)
    }
}

impl Drop for Gallery {
    fn drop(&mut self) {
        self.stop.swap(true, Ordering::Relaxed);
        self.inotify_thread
            .take()
            .expect("invariant: inotify_thread should never be None prior to Drop()")
            .join()
            .expect("failed to join the thread");
    }
}
