use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::str;

use anyhow::{anyhow, Result};
use serde::Deserialize;

pub fn load_config(config_path: &str) -> Result<GalleryConfig> {
    let config_file = File::open(config_path)?;
    let gallery_cfg = serde_json::from_reader(config_file)?;
    Ok(gallery_cfg)
}

// May exit.
pub fn parse_config_path_from_args_or_die() -> Result<String> {
    if let Some(first_arg) = env::args().nth(1) {
        Ok(first_arg)
    } else {
        Err(anyhow!("no configuration file argument specified"))
    }
}

#[derive(Deserialize, Debug)]
pub struct GalleryConfig {
    pub dirs: Vec<PathBuf>,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_port() -> u16 {
    8000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_json() {
        let load_res = load_config("./src/testconfigs/invalid_json.json");
        assert!(load_res.is_err());
        assert_eq!(
            load_res.err().unwrap().to_string(),
            "expected `,` or `]` at line 4 column 1"
        );
    }

    #[test]
    fn nonexistent_config() {
        let load_res = load_config("./src/testconfigs/i_dont_exist.json");
        assert!(load_res.is_err());
        assert_eq!(
            load_res.err().unwrap().to_string(),
            "No such file or directory (os error 2)"
        );
    }
}
