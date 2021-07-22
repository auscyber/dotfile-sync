use crate::file_actions::check_path;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    string::ParseError,
};

use cascade::cascade;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct System(String);

impl std::str::FromStr for System {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(System(s.into()))
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct VariablePath(String);

impl VariablePath {
    pub fn from_string(a: impl AsRef<str>) -> VariablePath {
        VariablePath(a.as_ref().to_owned())
    }
    pub fn from_path(a: impl AsRef<Path>) -> Result<VariablePath> {
        Ok(VariablePath(
            a.as_ref().canonicalize()?.to_string_lossy().into(),
        ))
    }

    pub fn to_path_buf(self, extra_variables: Option<HashMap<String, String>>) -> Result<PathBuf> {
        Ok(PathBuf::from(crate::util::parse_vars(
            true,
            extra_variables.as_ref(),
            self.0.as_str(),
        )?))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Link {
    pub name: String,
    pub destination: VariablePath,
    pub src: SourceFile,
}

impl Link {
    pub fn new(name: String, src: VariablePath, destination: SourceFile) -> Result<Link> {
        Ok(Link {
            name,
            destination: src,
            src: destination,
        })
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
#[serde(untagged)]
pub enum SourceFile {
    SourceWithNoSystem(String),
    SourceWithSystem(System, String),
    DynamicSourceWithDefaultPath(String, HashMap<System, String>),
    DynamicSourceWithDefaultSystem(System, HashMap<System, String>),
    DynamicSourceMap(HashMap<System, String>),
}

impl SourceFile {
    pub fn insert_link(self, sys: &System, dest_string: &str) -> Result<Self> {
        let dest_string = dest_string.to_owned();
        let sys = sys.clone();
        Ok(match self {
            SourceFile::SourceWithNoSystem(path) => {
                let map = cascade! {
                    HashMap::new();
                    ..insert(sys, dest_string.to_owned());
                };
                SourceFile::DynamicSourceWithDefaultPath(path, map)
            }
            SourceFile::SourceWithSystem(system, path) => {
                if system == sys {
                    bail!(
                        r#"System "{:?}" already defined for output file "{}" "#,
                        sys,
                        dest_string
                    );
                }
                let map = cascade! {
                    HashMap::new();
                    ..insert(system.clone(),path);
                    ..insert(sys,dest_string);
                };
                SourceFile::DynamicSourceWithDefaultSystem(system, map)
            }
            SourceFile::DynamicSourceWithDefaultPath(path, mut map) => {
                if map.contains_key(&sys) {
                    bail!(
                        r#"System "{:?}" already defined for output file "{}" "#,
                        sys,
                        dest_string
                    );
                }
                map.insert(sys, dest_string);
                SourceFile::DynamicSourceWithDefaultPath(path, map)
            }
            SourceFile::DynamicSourceWithDefaultSystem(system, mut map) => {
                if system == sys || map.contains_key(&system) {
                    bail!(
                        r#"System "{:?}" already defined for output file "{}" "#,
                        sys,
                        dest_string
                    );
                }
                map.insert(sys, dest_string);
                SourceFile::DynamicSourceWithDefaultSystem(system, map)
            }
            SourceFile::DynamicSourceMap(mut map) => {
                if map.contains_key(&sys) {
                    bail!(
                        r#"System "{:?}" already defined for output file "{}" "#,
                        sys,
                        dest_string
                    );
                }
                map.insert(sys, dest_string);
                SourceFile::DynamicSourceMap(map)
            }
        })
    }
    pub fn contains_path(&self, path: &str) -> bool {
        match self {
            SourceFile::SourceWithNoSystem(x) => path == x,
            SourceFile::SourceWithSystem(_, x) => path == x,
            SourceFile::DynamicSourceWithDefaultPath(x, xs) => {
                xs.iter().find(|(_, x)| *x == path).is_some() || x == path
            }
            SourceFile::DynamicSourceWithDefaultSystem(_, xs) => {
                xs.iter().find(|(_, x)| *x == path).is_some()
            }
            SourceFile::DynamicSourceMap(xs) => xs.iter().find(|(_, x)| *x == path).is_some(),
        }
    }

    pub fn resolve(&self, system: &Option<System>) -> Option<String> {
        let system = match system {
            None => {
                if let SourceFile::SourceWithNoSystem(path) = self {
                    return Some(path.clone());
                } else {
                    return None;
                }
            }
            Some(x) => x,
        };
        match self {
            SourceFile::SourceWithNoSystem(path) => Some(path.clone()),
            SourceFile::DynamicSourceMap(system_map) => system_map.get(&system).cloned(),
            SourceFile::DynamicSourceWithDefaultSystem(sys, system_map) => system_map
                .get(&system)
                .or_else(|| system_map.get(&sys))
                .cloned(),
            SourceFile::DynamicSourceWithDefaultPath(path, system_map) => {
                system_map.get(&system).or(Some(path)).cloned()
            }
            SourceFile::SourceWithSystem(default_system, default_map) => {
                if default_system == system {
                    Some(default_map.clone())
                } else {
                    None
                }
            }
        }
    }

    pub fn remove_link(self, search_path: &String) -> Option<SourceFile> {
        match self {
            SourceFile::SourceWithNoSystem(a) => {
                if search_path != &a {
                    Some(SourceFile::SourceWithNoSystem(a))
                } else {
                    None
                }
            }
            SourceFile::DynamicSourceMap(a) => {
                let map: HashMap<System, String> = a
                    .iter()
                    .filter_map(|(a, x)| {
                        if search_path != x {
                            Some((a.clone(), x.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                if map.is_empty() {
                    None
                } else {
                    Some(SourceFile::DynamicSourceMap(map))
                }
            }
            SourceFile::SourceWithSystem(sys, map) => {
                if search_path != &map {
                    Some(SourceFile::SourceWithSystem(sys, map))
                } else {
                    None
                }
            }
            SourceFile::DynamicSourceWithDefaultPath(path, map) => {
                if search_path == &path {
                    return Some(SourceFile::DynamicSourceMap(map));
                }
                let map = map
                    .into_iter()
                    .filter_map(|(k, x)| {
                        if search_path != x.as_str() {
                            Some((k, x))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<System, String>>();
                Some(SourceFile::DynamicSourceWithDefaultPath(path, map))
            }
            SourceFile::DynamicSourceWithDefaultSystem(def, map) => {
                let map: HashMap<System, String> = map
                    .iter()
                    .filter_map(|(a, x)| {
                        if search_path != x {
                            Some((a.clone(), x.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                if map.is_empty() {
                    None
                } else {
                    Some(SourceFile::DynamicSourceWithDefaultSystem(def, map))
                }
            }
        }
    }

    pub fn new(base_url: &PathBuf, dest: String) -> Result<SourceFile> {
        Ok(SourceFile::SourceWithNoSystem(dest.clone()))
    }

    pub fn with_default(
        base_url: &PathBuf,
        default: String,
        system_map: HashMap<System, String>,
    ) -> Result<SourceFile> {
        let mut new_map: HashMap<System, String> = system_map
            .iter()
            .map(move |(key, elem)| {
                check_path(&base_url.join(elem))?;
                Ok((key.clone(), elem.clone()))
            })
            .collect::<Result<HashMap<System, String>>>()?;

        Ok(SourceFile::DynamicSourceWithDefaultSystem(
            System(default),
            new_map,
        ))
    }
}
