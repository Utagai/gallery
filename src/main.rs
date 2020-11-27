#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use std::env;
use std::fs::File;
use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct GalleryConfig {
    dirs: Vec<PathBuf>,
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn load_config(config_path: &str) -> Result<GalleryConfig> {
    let config_file = File::open(config_path)?;
    let gallery_cfg = serde_json::from_reader(config_file)?;
    Ok(gallery_cfg)
}

// May exit.
fn parse_config_path_from_args_or_die() -> Result<String> {
    if let Some(first_arg) = env::args().nth(1) {
        Ok(first_arg)
    } else {
        Err(Error::msg("no configuration file argument specified"))
    }
}

fn main() -> Result<()> {
    let config_filepath =
        parse_config_path_from_args_or_die().context("failed to open the config file")?;
    let gallery_cfg = load_config(&config_filepath).context("failed to parse config")?;
    println!("gallery cfg: {:?}", gallery_cfg);
    rocket::ignite().mount("/", routes![index]).launch();

    Ok(())
}
