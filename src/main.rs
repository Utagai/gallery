#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use anyhow::{Context, Result};

mod config;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn main() -> Result<()> {
    let config_filepath =
        config::parse_config_path_from_args_or_die().context("failed to open the config file")?;
    let gallery_cfg = config::load_config(&config_filepath).context("failed to parse config")?;
    println!("gallery cfg: {:?}", gallery_cfg);

    rocket::ignite().mount("/", routes![index]).launch();

    Ok(())
}
