use ant_metadata::get_typesofants_home;
use anyhow::{Ok, Result};

pub fn initialize() -> Result<()> {
    std::fs::create_dir_all(get_typesofants_home())?;

    Ok(())
}
