#![feature(proc_macro_hygiene, decl_macro)]

use std::path::Path;
use std::fs::File;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_include_static_resources;

use anyhow::{Context, Result};
use rocket::response::Stream;
use rocket::{Config, config::Environment, Rocket, State};
use rocket_contrib::templates::Template;
use rocket_include_static_resources::StaticResponse;

mod config;
mod gallery;

#[get("/img?<path>")]
fn get_img(gallery: State<gallery::Gallery>, path: String) -> Option<Stream<File>> {
    let p = Path::new(&path);
    if !gallery.has(p) {
        return None;
    }

    File::open(p).map(|file| Stream::from(file)).ok()
}

#[get("/thumbnail?<path>")]
fn get_thumbnail(gallery: State<gallery::Gallery>, path: String) -> Option<Stream<File>> {
    let p = Path::new(&path);
    if !gallery.has(p) {
        return None;
    }

    let thumbnail_path = gallery.get_thumbnail_path(p);
    File::open(thumbnail_path).map(|file| Stream::from(file)).ok()
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

#[get("/favicon.ico")]
fn favicon() -> StaticResponse {
    static_response!("favicon")
}

#[get("/favicon-16.png")]
fn favicon_png() -> StaticResponse {
    static_response!("favicon-png")
}

fn rocket(gallery: gallery::Gallery, cfg: Config) -> Rocket {
    rocket::custom(cfg)
        .attach(StaticResponse::fairing(|resources| {
            static_resources_initialize!(
                resources,
                "favicon", "./rsrc/favicon/favicon.ico",
                "favicon-png", "./rsrc/favicon/favicon.png",
            );
        }))
        .mount("/", routes![index, get_thumbnail, get_img, favicon, favicon_png])
        .attach(Template::fairing())
        .manage(gallery)
}

fn main() -> Result<()> {
    let config_filepath =
        config::parse_config_path_from_args_or_die().context("failed to open the config file")?;
    let gallery_cfg = config::load_config(&config_filepath).context("failed to parse config")?;

    let gallery =
        gallery::Gallery::new(&gallery_cfg).context("could not scan image directories")?;

    let rocket_cfg = Config::build(Environment::Production).
        port(gallery_cfg.port).
        log_level(gallery_cfg.get_rocket_logging_level()).
        unwrap();

    rocket(gallery, rocket_cfg).launch();

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;
    use rocket::http::Status;
    use rocket::local::Client;
    use rocket::config::LoggingLevel;
    use scraper::{Html, Selector};
    use std::fs;

    // We don't actually _need_ to have a separate port for these tests, since we are using
    // rocket::local::Client which is not networked, but whatever.
    static TEST_ROCKET_PORT: u16 = 8002;
    static TEST_ROCKET_LOG_LEVEL: LoggingLevel = LoggingLevel::Debug;

    fn gallery() -> gallery::Gallery {
        let gallery_cfg = config::load_config("./testdata/cfgs/rocket_tests.json")
            .expect("failed to load the rocket tests configuration");
        gallery::Gallery::new(&gallery_cfg).expect("could not create the Gallery")
    }

    fn get_num_imgs_rendered(mut response: rocket::local::LocalResponse) -> usize {
        let html_text = response
            .body_string()
            .expect("did not get any response body");
        let document = Html::parse_document(&html_text);
        let selector = Selector::parse("div.image").expect("failed to parse image selector");
        document.select(&selector).count()
    }

    #[test]
    fn index_page_has_right_num_of_imgs() {
        let gallery = gallery();
        let rocket_cfg = Config::build(Environment::Production).
            port(TEST_ROCKET_PORT).
            log_level(TEST_ROCKET_LOG_LEVEL).
            unwrap();
        let client = Client::new(rocket(gallery, rocket_cfg)).expect("valid rocket instance");
        let response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);

        // There are 2 images under the directory configured in the test config.
        // Note that there is also another image in a nested directory, but it is not picked up
        // because we ignore nested images. Therefore, this test also confirms that behavior.
        assert_eq!(get_num_imgs_rendered(response), 2);
    }

    #[test]
    fn returned_image_is_correct() {
        let gallery = gallery();
        let rocket_cfg = Config::build(Environment::Production).
            port(TEST_ROCKET_PORT).
            log_level(TEST_ROCKET_LOG_LEVEL).
            unwrap();
        let client = Client::new(rocket(gallery, rocket_cfg)).expect("valid rocket instance");
        let img_path = "./testdata/pics/2.png";
        let mut response = client.get(format!("/img?path={}", img_path)).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let actual_bytes = response
            .body_bytes()
            .expect("did not get any response body bytes");
        let expected_bytes = fs::read(img_path).expect("failed to read image from disk");
        let zipper = actual_bytes.iter().zip(expected_bytes.iter());
        for (actual, expected) in zipper {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn file_not_in_gallery_is_rejected() {
        let gallery = gallery();
        let rocket_cfg = Config::build(Environment::Production).
            port(TEST_ROCKET_PORT).
            log_level(TEST_ROCKET_LOG_LEVEL).
            unwrap();
        let client = Client::new(rocket(gallery, rocket_cfg)).expect("valid rocket instance");
        let img_path = "/home/oblivious_bob/.ssh/id_rsa";
        let response = client.get(format!("/img?path={}", img_path)).dispatch();
        assert_eq!(response.status(), Status::NotFound);
        let response = client.get(format!("/thumbnail?path={}", img_path)).dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    fn added_and_removed_images_are_detected() {
        let gallery = gallery();
        // Compute these now before we move gallery into `rocket()`.
        let new_file_path = "./testdata/pics/3.png";
        let thumbnail_path = gallery.get_thumbnail_path(Path::new(&new_file_path));
        let rocket_cfg = Config::build(Environment::Production).
            port(TEST_ROCKET_PORT).
            log_level(TEST_ROCKET_LOG_LEVEL).
            unwrap();
        let client = Client::new(rocket(gallery, rocket_cfg)).expect("valid rocket instance");
        let response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        // There are 2 images under the directory configured in the test config.
        assert_eq!(get_num_imgs_rendered(response), 2);

        // Now cp a pre-existing file into the tracked directory, and expect 3 images.
        let img_path = "./testdata/pics/2.png";
        let bytes_to_copy = fs::read(img_path).expect("failed to read image from disk");
        fs::write(new_file_path, bytes_to_copy).expect("failed to copy over bytes");

        // This is a bit flaky, but, since the Gallery does not instantly learn about filesystem
        // changes, we need to give it some time to notice the change.
        // We multiply the periodicity by 2 to get a very generous amount of padding.
        std::thread::sleep(gallery::Gallery::periodicity() * 2);
        assert_eq!(get_num_imgs_rendered(client.get("/").dispatch()), 3);
        // And check that a thumbnail has been created:
        assert!(thumbnail_path.exists());

        // Now, delete that new file and expect a return to 2 images.
        fs::remove_file(new_file_path).expect("failed to remove the copied file");
        fs::remove_file(thumbnail_path).expect("failed to remove the thumbnail file");
        // Ditto comment above about the other sleep.
        std::thread::sleep(gallery::Gallery::periodicity() * 2);
        assert_eq!(get_num_imgs_rendered(client.get("/").dispatch()), 2);
    }
}
