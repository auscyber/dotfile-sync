use snafu::{Snafu, ResultExt};
use std::{collections::hash_map, env};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParsingVarError {
    #[error("Internal Var Error: {0}")]
    VarError(#[from] std::env::VarError),
    #[error("Could not find var `{0}`")]
    NotFound(String),
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
                    Err(ParsingVarError::NotFound(text.to_string()))
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

use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
#[derive(Snafu, Debug)]
pub enum WritableConfigError {
    #[snafu(display("Unable to read {}: {}",path.display(),source))]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Unable to write {}: {}",path.display(),source))]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Unable to deserialize {}: {}",path.display(),source))]
    TomlDe {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[snafu(display("Unable to serialize {}: {}",path.display(),source))]
    TomlSer {
        path: PathBuf,
        source: toml::ser::Error,
    },
}

#[async_trait::async_trait]
pub trait WritableConfig: Sized {
    fn write_to_file(&self, file_name: &Path) -> Result<(), WritableConfigError>;
    fn read_from_file(file_name: &Path) -> Result<Self, WritableConfigError>;
}

impl<T: Sync + DeserializeOwned + Send + Serialize + Clone> WritableConfig for T {
    fn write_to_file(&self, path: &Path) -> Result<(), WritableConfigError> {
        let data = toml::to_vec(self).context(TomlSerSnafu { path})?;
        fs::write(path, &data).context(WriteSnafu{path})?;
        Ok(())
    }
    fn read_from_file(path: &Path) -> Result<Self, WritableConfigError> {
        let data = fs::read(path).context(ReadSnafu{path})?;
        let val = toml::from_slice(&data).context(TomlDeSnafu { path})?;
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
