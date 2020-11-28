#[cfg(test)]
use pretty_assertions::assert_eq;
use std::process::Command;
use std::str;

static BINARY: &str = "./target/debug/gallery";

#[test]
fn no_config_file_specified() {
    let output = Command::new(BINARY)
        .output()
        .expect("failed to start the server");

    let stderr_output = str::from_utf8(&output.stderr).expect("could not encode stderr as UTF-8");

    assert_eq!(
        stderr_output,
        concat!(
            "Error: failed to open the config file\n\n",
            "Caused by:\n",
            "    no configuration file argument specified\n",
        )
    )
}

#[test]
fn nonexistent_directory() {
    let output = Command::new(BINARY)
        .arg("./tests/configs/nonexistent_dir.json")
        .output()
        .expect("failed to start the server");

    let stderr_output = str::from_utf8(&output.stderr).expect("could not encode stderr as UTF-8");

    assert_eq!(
        stderr_output,
        concat!(
            "Error: could not scan image directories\n\n",
            "Caused by:\n",
            "    0: failed to open directory \'./foo/i_dont_exist\'\n",
            "    1: No such file or directory (os error 2)\n"
        )
    )
}
