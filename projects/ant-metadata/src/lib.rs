use std::path::PathBuf;

mod common;
pub use common::*;

pub fn get_typesofants_home() -> PathBuf {
    let home = home::home_dir().unwrap();
    [home, ".typesofants".into()].iter().collect()
}
