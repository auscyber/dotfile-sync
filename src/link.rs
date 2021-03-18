use crate::file_actions::check_path;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, path, path::PathBuf};
pub type System = String;
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Link {
    pub name: String,
    pub src: PathBuf,
    pub destination: Destination,
}

impl Link {
    pub fn new(name: String, src: String, destination: Destination) -> Result<Link> {
        Ok(Link { name, src: crate::file_actions::check_path(&path::Path::new(&src).to_path_buf())?, destination })
    }
}
#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
#[serde(untagged)]
pub enum Destination {
    DefaultDest(String),
    SystemDest(System, String),
    DynamicDestinationWithDefault(System, HashMap<System, String>),
    DynamicDestination(HashMap<System, String>),
}

impl Destination {
    pub fn new(base_url: &PathBuf, dest: String) -> Result<Destination> {
        check_path(&base_url.join(&dest))?;
        Ok(Destination::DefaultDest(dest.clone()))
    }

    pub fn with_default(base_url: &PathBuf, default: String, system_map: HashMap<System, String>) -> Result<Destination> {
        let mut new_map: HashMap<System, String> = system_map
            .iter()
            .map(move |(key, elem)| {
                check_path(&base_url.join(elem))?;
                Ok((key.clone(), elem.clone()))
            })
            .collect::<Result<HashMap<System, String>>>()?;

        Ok(Destination::DynamicDestinationWithDefault(default, new_map))
    }
}
