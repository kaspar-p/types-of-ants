use std::{collections::HashMap, path::Path};

use anyhow::Context;

pub fn escape_env_variable(val: &str) -> String {
    let val = val.replace("\"", "\\\"");
    format!("\"{}\"", val)
}

pub fn env_vars_to_map(path: &Path) -> Result<HashMap<String, String>, anyhow::Error> {
    let mut variables = HashMap::<String, String>::new();
    let entries = match dotenvy::from_path_iter(&path) {
        Err(dotenvy::Error::Io(io_err))
            if matches!(io_err.kind(), std::io::ErrorKind::NotFound) =>
        {
            Ok(vec![])
        }

        Err(e) => Err(e).context(format!("reading env: {}", path.display())),
        Ok(f) => Ok(f
            .into_iter()
            .filter_map(|e| match e {
                Err(_) => None,
                Ok(t) => Some(t),
            })
            .collect::<Vec<(String, String)>>()),
    }?;

    for (k, v) in entries {
        variables.insert(k, v);
    }

    Ok(variables)
}
