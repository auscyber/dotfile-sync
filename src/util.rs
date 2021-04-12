use anyhow::*;
use regex::Captures;
use std::{collections::hash_map, env, fs};
use structopt::lazy_static::lazy;

pub fn create_folders(path: impl AsRef<std::path::Path>) -> Result<()> {
    match fs::DirBuilder::new()
        .recursive(true)
        .create(match path.as_ref().parent() {
            Some(x) => x,
            None => return Ok(()),
        }) {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
        lol => lol?,
    }
    Ok(())
}

pub fn parse_vars(
    use_env: bool,
    extra_map: Option<hash_map::HashMap<String, String>>,
    text: &str,
) -> Result<String> {
    let get_variables: regex::Regex = regex::Regex::new(r"(?:\$(\w+))|(?:\$\{(\w+?)\})")?;
    let value = get_variables
        .replace_all(text, |x: &Captures| {
            let var_name = x.get(1).or_else(|| x.get(2)).map(|x| x.as_str()).unwrap();
            if let Some(a) = extra_map.clone() {
                a.get(var_name).map(|x| x.clone())
            } else {
                None
            }
            .or_else(|| {
                if use_env {
                    log::debug!("{}", var_name);
                    let env_var = env::var(var_name).ok()?;
                    log::debug!("{}", env_var);
                    Some(env_var)
                } else {
                    None
                }
            })
            .unwrap()
            .clone()
        })
        .into();
    log::debug!("{}", value);
    Ok(value)
}
