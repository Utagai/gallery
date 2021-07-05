use std::collections::HashMap;
use std::fs::{create_dir_all, read_dir, DirEntry, ReadDir};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

use anyhow::{bail, Context, Result};
use dirs;
use image::io::Reader as ImageReader;
use inotify::{EventMask, Inotify, WatchMask};
use serde::Serialize;

use crate::config;

#[derive(Debug)]
pub struct Gallery {
    pub paths: Arc<Mutex<Vec<PathBuf>>>,
    cache_dir: PathBuf,
    stop: Arc<AtomicBool>,
    inotify_thread: Option<JoinHandle<Result<()>>>,
}

static GALLERY_CACHE_DIR_NAME: &'static str = "gallery";
static THUMBNAIL_FORMAT: image::ImageFormat = image::ImageFormat::Jpeg;
// static THUMBNAIL_FORMAT_EXT: &'static str =
fn thumbnail_format_ext() -> &'static str {
    THUMBNAIL_FORMAT
        .extensions_str()
        .first()
        // This should never panic really... I think hehe.
        .expect(
            "expected there to be at least one extension string for the chosen thumbnail format",
        )
}
// Maintain a 16:9 aspect ratio.
// Note that since we are using .thumbnail(), the `image` crate will preserve the aspect ratio of
// the original image anyways, so this isn't really a big deal.
static THUMBNAIL_WIDTH: u32 = 16 * 20;
static THUMBNAIL_HEIGHT: u32 = 9 * 20;

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

        let paths_vec = results
            .iter()
            .map(|x| x.path())
            .filter(|x| !x.is_dir()) // We do not support nested directories.
            .collect::<Vec<PathBuf>>();

        if let Some(cache_dir) = dirs::cache_dir() {
            let gallery_cache_dir = cache_dir.join(GALLERY_CACHE_DIR_NAME);
            // Now that we found the cache dir, let's make a gallery subdir:
            create_dir_all(&gallery_cache_dir)?;
            Gallery::make_thumbnails(&gallery_cache_dir, &paths_vec)?;

            let stop = Arc::new(AtomicBool::new(false));
            let stop_clone = stop.clone();
            let dirs_clone = cfg.dirs.to_vec();
            let paths = Arc::new(Mutex::new(paths_vec));
            let paths_clone = paths.clone();
            let gallery_cache_dir_copy = gallery_cache_dir.clone();
            let inotify_thread = Some(thread::spawn(move || -> Result<()> {
                Gallery::reactor(dirs_clone, paths_clone, &gallery_cache_dir_copy, stop_clone)
            }));

            Ok(Gallery {
                paths,
                cache_dir: gallery_cache_dir,
                stop,
                inotify_thread,
            })
        } else {
            bail!("failed to find the cache dir");
        }
    }

    fn make_thumbnails(gallery_cache_dir: &PathBuf, paths: &Vec<PathBuf>) -> Result<()> {
        for path in paths {
            let pathref = &path;
            if let Some(img_filename) = path.file_name() {
                let mut thumbnail_path = gallery_cache_dir.join(img_filename);
                // Unfortunately, we need to explicitly set the extension here because the
                // image crate does not support saving files of every single format that
                // gallery could potentially return.
                if !thumbnail_path.set_extension(thumbnail_format_ext()) {
                    bail!(
                        "failed to set the thumbnail path '{}' to be .{}",
                        // Blissfully assume this conversion will always work.
                        thumbnail_path.to_str().unwrap(),
                        thumbnail_format_ext(),
                    );
                }
                if thumbnail_path.exists() {
                    // Don't waste time creating this thumbnail if it exists already.
                    continue;
                }
                let img = ImageReader::open(pathref)?.decode()?;
                let thumbnail = img.thumbnail(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT);
                thumbnail.save_with_format(thumbnail_path, image::ImageFormat::Png)?;
            }
        }
        return Ok(());
    }

    // The amount of time the Gallery will wait before it checks for filesystem changes again.
    pub fn periodicity() -> std::time::Duration {
        std::time::Duration::from_millis(1000)
    }

    fn reactor(
        dirs: Vec<PathBuf>,
        paths: Arc<Mutex<Vec<PathBuf>>>,
        cache_dir: &PathBuf,
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

                Gallery::make_thumbnails(cache_dir, &mut_paths)?;
            }

            // Don't hammer the CPU.
            drop(mut_paths);
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

    pub fn get_thumbnail_path(&self, path: &Path) -> PathBuf {
        let mut path_copy = path.to_path_buf();
        path_copy.set_extension(thumbnail_format_ext());
        self.cache_dir.join(path_copy.file_name().unwrap())
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
        // Nothing we can do if this happens, so just print something out.
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
