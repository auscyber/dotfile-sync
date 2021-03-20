use crate::file_actions::check_path;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, path, path::PathBuf, string::ParseError};

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct System(String);

impl std::str::FromStr for System {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(System(s.into()))
    }
}

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
    pub fn resolve(self, system: &System) -> Option<String> {
        match self {
            Destination::DefaultDest(path) => Some(path),
            Destination::DynamicDestination(system_map) => system_map.get(system).cloned(),
            Destination::DynamicDestinationWithDefault(path, system_map) => {
                system_map.get(system).or_else(|| system_map.get(&path)).cloned()
            }
            Destination::SystemDest(default_system, default_map) => {
                if &default_system == system {
                    Some(default_map.clone())
                } else {
                    None
                }
            }
        }
    }

    pub fn remove_link(self, link: &String) -> Option<Destination> {
        match self {
            Destination::DefaultDest(a) => {
                if link != &a {
                    Some(Destination::DefaultDest(a))
                } else {
                    None
                }
            }
            Destination::DynamicDestination(a) => {
                let map: HashMap<System, String> =
                    a.iter().filter_map(|(a, x)| if link != x { Some((a.clone(), x.clone())) } else { None }).collect();
                if map.len() == 0 {
                    None
                } else {
                    Some(Destination::DynamicDestination(map))
                }
            }
            Destination::SystemDest(sys, a) => {
                if link != &a {
                    Some(Destination::SystemDest(sys, a))
                } else {
                    None
                }
            }
            Destination::DynamicDestinationWithDefault(def, a) => {
                let map: HashMap<System, String> =
                    a.iter().filter_map(|(a, x)| if link != x { Some((a.clone(), x.clone())) } else { None }).collect();
                if map.len() == 0 {
                    None
                } else {
                    Some(Destination::DynamicDestinationWithDefault(def, map))
                }
            }
        }
    }

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

        Ok(Destination::DynamicDestinationWithDefault(System(default), new_map))
    }
}
