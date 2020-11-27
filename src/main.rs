#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use anyhow::{Context, Result};
use rocket::State;
use rocket_contrib::templates::Template;

mod config;

#[get("/")]
fn index(gallery: State<config::Gallery>) -> Template {
    // TODO: This is not as performant as encoding an API endpoint that returns the image bytes.
    // Doing the API endpoint method lets us get natural parallelism, as the browser will fire off
    // a request for each image individually, whereas, currently, we are serially writing the bytes
    // of the images into the HTML file.
    //
    // That said, allowing endpoints gives access to the disk in some capacity, which is a big
    // security concern. Likely, we can get it done securely by only returning file URLs that the
    // gallery is tracking.
    //
    // That said, I am risk averse enough (and dumb enough) to not trust myself to write that code
    // correctly, so I'll tolerate the slower page load. Perhaps when I use this with enough
    // images, it'll take so damn long that I'll bite the bullet. Right now, I'm not there yet.
    Template::render("index", gallery.inner())
}

fn main() -> Result<()> {
    let config_filepath =
        config::parse_config_path_from_args_or_die().context("failed to open the config file")?;
    let gallery_cfg = config::load_config(&config_filepath).context("failed to parse config")?;

    let gallery = config::Gallery::new(&gallery_cfg).context("could not scan image directories")?;

    rocket::ignite()
        .mount("/", routes![index])
        .attach(Template::fairing())
        .manage(gallery)
        .launch();

    Ok(())
}
