#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use anyhow::{Context, Result};
use rocket::State;

mod config;

#[get("/")]
fn index(gallery: State<config::Gallery>) -> String {
    format!("Hello, world!: {:?}", gallery)
}

fn main() -> Result<()> {
    let config_filepath =
        config::parse_config_path_from_args_or_die().context("failed to open the config file")?;
    let gallery_cfg = config::load_config(&config_filepath).context("failed to parse config")?;

    let gallery = config::Gallery::new(&gallery_cfg).context("could not scan image directories")?;

    rocket::ignite()
        .mount("/", routes![index])
        .manage(gallery)
        .launch();

    Ok(())
}
