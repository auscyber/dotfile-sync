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

#[derive(Debug)]
pub enum ParsingVarError {
    VarNotFound(std::env::VarError),
    Other(anyhow::Error),
}

impl From<std::env::VarError> for ParsingVarError {
    fn from(error: std::env::VarError) -> Self {
        ParsingVarError::VarNotFound(error)
    }
}
impl From<anyhow::Error> for ParsingVarError {
    fn from(e: anyhow::Error) -> Self {
        ParsingVarError::Other(e)
    }
}

impl From<ParsingVarError> for anyhow::Error {
    fn from(e: ParsingVarError) -> anyhow::Error {
        match e {
            ParsingVarError::VarNotFound(e) => anyhow::Error::new(e),
            ParsingVarError::Other(e) => e,
        }
    }
}

pub fn parse_vars(
    use_env: bool,
    extra_map: Option<&hash_map::HashMap<String, String>>,
    text: &str,
) -> Result<String, ParsingVarError> {
    lazy_static::lazy_static! {
        static ref GET_VARIABLES: regex::Regex = regex::Regex::new(r"(?:\$(\w+))|(?:\$\{(\w+?)\})").unwrap();
    }
    let mut extra_offset: i32 = 0;
    let mut output = text.to_string();
    for captures in GET_VARIABLES.captures_iter(text) {
        let matches = captures.get(1).or_else(|| captures.get(2)).unwrap();
        let offsets = captures.get(0).or_else(|| captures.get(1)).unwrap();
        let text = matches.as_str();
        println!("{}", text);
        let variable_value = extra_map
            .as_ref()
            .and_then(|x| x.get(text).cloned())
            .context("Could not get extra variables from map")
            .or_else(|_| {
                if use_env {
                    Ok(env::var(text)?)
                } else {
                    Err(ParsingVarError::Other(anyhow::Error::msg(format!(
                        "Could not find extra variable {}",
                        text
                    ))))
                }
            })?;
        output.replace_range(
            ((offsets.start() as i32 + extra_offset) as usize)
                ..((offsets.end() as i32 + extra_offset) as usize),
            &variable_value,
        );
        extra_offset += variable_value.as_str().len() as i32 - offsets.as_str().len() as i32;
    }
    Ok(output)
}
