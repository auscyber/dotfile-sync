use anyhow::{Context, Result};
use std::{collections::hash_map, env};

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
    core::result::Result::Ok(output)
}

use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;
#[async_trait::async_trait]
pub trait WritableConfig: Sized {
    fn write_to_file(&self, file_name: &Path) -> Result<()>;
    fn read_from_file(file_name: &Path) -> Result<Self>;
}

impl<T: Sync + DeserializeOwned + Send + Serialize + Clone> WritableConfig for T {
    fn write_to_file(&self, path: &Path) -> Result<()> {
        let data = toml::to_vec(self)?;
        fs::write(path, &data)?;
        Ok(())
    }
    fn read_from_file(file_name: &Path) -> Result<Self> {
        let data = fs::read(file_name)?;
        let val = toml::from_slice(&data)?;
        Ok(val)
    }
}

use std::ffi::OsStr;
use std::process::Stdio;
use tokio::process::Command;
pub fn run_command<I, S>(program: &str, args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::inherit());
    command
}
