use command_macros::cmd;
use std::path::Path;

pub async fn build(_root: &Path) -> () {
    // TODO: build this, then place the artifacts in a known location
    cmd!(cargo build).spawn().unwrap();
    println!("Finished!");
}
