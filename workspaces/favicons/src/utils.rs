use std::{env, path::PathBuf};

pub const FAVICONS_DIRECTORY: &str = "favicons";

pub fn get_favicons_directory() -> PathBuf {
    let cwd = env::current_dir().expect("Failed to get the cwd");
    cwd.join(FAVICONS_DIRECTORY)
}
