use std::collections::HashMap;
use std::fs::{read_dir, DirEntry, ReadDir};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use inotify::{EventMask, Inotify, WatchMask};
use serde::Serialize;

use crate::config;

#[derive(Debug)]
pub struct Gallery {
    pub paths: Arc<Mutex<Vec<PathBuf>>>,
    stop: Arc<AtomicBool>,
    inotify_thread: Option<JoinHandle<Result<()>>>,
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

        let path_vec = results
            .iter()
            .map(|x| x.path())
            .filter(|x| !x.is_dir()) // We do not support nested directories.
            .collect::<Vec<PathBuf>>();

        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let dirs_clone = cfg.dirs.to_vec();
        let paths = Arc::new(Mutex::new(path_vec));
        let paths_clone = paths.clone();
        let inotify_thread = Some(thread::spawn(move || -> Result<()> {
            Gallery::reactor(dirs_clone, paths_clone, stop_clone)
        }));

        Ok(Gallery {
            paths,
            stop,
            inotify_thread,
        })
    }

    // The amount of time the Gallery will wait before it checks for filesystem changes again.
    pub fn periodicity() -> std::time::Duration {
        std::time::Duration::from_millis(200)
    }

    fn reactor(
        dirs: Vec<PathBuf>,
        paths: Arc<Mutex<Vec<PathBuf>>>,
        stop: Arc<AtomicBool>,
    ) -> Result<()> {
        // This is a serious enough problem that I'd rather panic.
        let mut inotify = Inotify::init().expect("failed to initialize inotify");

        let mut watches = HashMap::new();
        for dir in dirs {
            let watch = inotify.add_watch(dir.as_path(), WatchMask::CREATE | WatchMask::DELETE)?;
            watches.insert(watch, dir.clone());
        }

        let mut buffer = [0u8; 4096];
        while !stop.load(Ordering::Relaxed) {
            let events = inotify.read_events(&mut buffer)?;
            let mut mut_paths = paths.lock().unwrap();
            for event in events {
                let file_name = match event.name {
                    Some(name) => name,
                    // This should likely never happen, as this is only true if the affected
                    // file is the watched directory/file itself. In either case, it calls for
                    // a skip.
                    None => continue,
                };
                let path = PathBuf::from(file_name);
                let dir = &watches[&event.wd];
                let mut dirpath = dir.clone();
                dirpath.push(path);

                if event.mask.contains(EventMask::ISDIR) {
                    // We don't support nested directory structures.
                    // We could... but I personally have no use for them.
                    continue;
                }

                if event.mask.contains(EventMask::CREATE) {
                    mut_paths.push(dirpath);
                } else if event.mask.contains(EventMask::DELETE) {
                    mut_paths.retain(|p| p != &dirpath);
                } else {
                    panic!("should not have received any inotify events besides CREATE/DELETE")
                }
            }

            // Don't hammer the CPU.
            thread::sleep(Gallery::periodicity());
        }

        Ok(())
    }

    // Returns a Vector that is a snapshot of the current Gallery.
    pub fn snapshot(&self) -> GallerySnapshot {
        GallerySnapshot {
            paths: self.paths.lock().unwrap().to_vec(),
        }
    }

    pub fn has(&self, path: &Path) -> bool {
        self.paths.lock().unwrap().iter().any(|p| p == path)
    }
}

impl Drop for Gallery {
    fn drop(&mut self) {
        self.stop.swap(true, Ordering::Relaxed);
        let res = self
            .inotify_thread
            .take()
            .expect("invariant: inotify_thread should never be None prior to Drop()")
            .join()
            .expect("failed to join the thread");

        // This is the final result of the thread, if there was any.
        if res.is_err() {
            println!("Thread reported error: {:?}", res.err());
        }
    }
}

// A snapshot of a Gallery in time, that does not change over time.
#[derive(Debug, Serialize)]
pub struct GallerySnapshot {
    pub paths: Vec<PathBuf>,
}
