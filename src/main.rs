#![feature(proc_macro_hygiene, decl_macro)]

use std::path::Path;

#[macro_use]
extern crate rocket;

use anyhow::{anyhow, Context, Error, Result};
use rocket::http::Status;
use rocket::response::{self, status::Custom, NamedFile, Responder};
use rocket::{Request, State};
use rocket_contrib::serve::{Options, StaticFiles};
use rocket_contrib::templates::Template;

mod config;
mod gallery;

struct GetImgResponder {
    res: Result<NamedFile>,
}

impl GetImgResponder {
    fn ok(res: NamedFile) -> GetImgResponder {
        GetImgResponder { res: Ok(res) }
    }

    fn err(err: Error) -> GetImgResponder {
        GetImgResponder { res: Err(err) }
    }
}

impl<'r> Responder<'r> for GetImgResponder {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        match self.res {
            Ok(named_file) => named_file.respond_to(req),
            Err(err) => {
                let resp = Custom(Status::BadRequest, format!("{}", err));
                resp.respond_to(req)
            }
        }
    }
}

#[get("/img?<path>")]
fn get_img(gallery: State<gallery::Gallery>, path: String) -> GetImgResponder {
    let p = Path::new(&path);
    if !gallery.has(p) {
        return GetImgResponder::err(anyhow!("'{}' is not in the gallery", path));
    }

    match NamedFile::open(p) {
        Ok(named_file) => GetImgResponder::ok(named_file),
        Err(err) => GetImgResponder::err(Error::new(err)),
    }
}

#[get("/")]
fn index(gallery: State<gallery::Gallery>) -> Template {
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
    Template::render("index", gallery.inner().snapshot())
}

fn main() -> Result<()> {
    let config_filepath =
        config::parse_config_path_from_args_or_die().context("failed to open the config file")?;
    let gallery_cfg = config::load_config(&config_filepath).context("failed to parse config")?;

    let gallery =
        gallery::Gallery::new(&gallery_cfg).context("could not scan image directories")?;

    rocket::ignite()
        .mount(
            "/favicon",
            StaticFiles::new(
                concat!(env!("CARGO_MANIFEST_DIR"), "./rsrc/favicon/"),
                Options::None,
            ),
        )
        .mount("/", routes![index, get_img])
        .attach(Template::fairing())
        .manage(gallery)
        .launch();

    Ok(())
}
