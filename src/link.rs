use crate::file_actions::check_path;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    string::ParseError,
};

use cascade::cascade;
use colored::*;
use derive_more::Display;
use itertools::Itertools;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize, Display)]
#[serde(transparent)]
pub struct System(String);

impl std::str::FromStr for System {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(System(s.into()))
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone, Display)]
#[serde(transparent)]
pub struct VariablePath(String);

impl<T: AsRef<str>> From<T> for VariablePath {
    fn from(a: T) -> Self {
        VariablePath(a.as_ref().to_owned())
    }
}

impl VariablePath {
    pub fn from_path(a: impl AsRef<Path>) -> Result<VariablePath> {
        Ok(VariablePath(
            a.as_ref().canonicalize()?.to_string_lossy().into(),
        ))
    }

    pub fn to_path_buf(
        &self,
        extra_variables: Option<&HashMap<String, String>>,
    ) -> Result<PathBuf> {
        Ok(PathBuf::from(crate::util::parse_vars(
            true,
            extra_variables,
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
impl std::fmt::Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {} Destination: {} Link Sources: \n {}",
            self.name.yellow(),
            format!("\"{}\"", self.destination).green(),
            format!("{}", self.src).red()
        )?;
        Ok(())
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
impl IntoIterator for SourceFile {
    type Item = (bool, Option<System>, String);
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;
    fn into_iter(self) -> Self::IntoIter {
        use SourceFile::*;
        match self {
            SourceWithNoSystem(path) => Box::new(Some((false, None, path)).into_iter()),
            SourceWithSystem(sys, path) => Box::new(Some((false, Some(sys), path)).into_iter()),
            DynamicSourceWithDefaultPath(path, map) => Box::new(
                Some((true, None, path))
                    .into_iter()
                    .chain(map.into_iter().map(|(sys, path)| (false, Some(sys), path))),
            ),
            DynamicSourceWithDefaultSystem(_, map) => {
                Box::new(map.into_iter().map(|(sys, path)| (true, Some(sys), path)))
            }
            DynamicSourceMap(map) => {
                Box::new(map.into_iter().map(|(sys, path)| (false, Some(sys), path)))
            }
        }
    }
}
impl fmt::Display for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceFile::SourceWithNoSystem(path) => writeln!(f, "\tPath: {}", path),
            SourceFile::SourceWithSystem(sys, path) => {
                writeln!(f, "\tSystem: {}, Path: {} ", sys, path)
            }
            SourceFile::DynamicSourceMap(map) => map
                .iter()
                .try_for_each(|(system, path)| writeln!(f, "\tSystem: {}, Path: {}", system, path)),
            SourceFile::DynamicSourceWithDefaultPath(path, map) => {
                writeln!(f, "\tDefault Path: {}", path)?;
                map.iter().try_for_each(|(system, path)| {
                    writeln!(f, "\tSystem: {}, Path: {}", system, path)
                })
            }
            SourceFile::DynamicSourceWithDefaultSystem(system, map) => {
                writeln!(
                    f,
                    "\tDefault system: {}   Default Path: {}",
                    system,
                    map.get(system).unwrap()
                )?;
                map.iter().try_for_each(|(system, path)| {
                    writeln!(f, "\tSystem: {:?}, Path: {}", system, path)
                })
            }
        }
    }
}

pub fn convert_iter_to_source<T: Iterator<Item = (bool, Option<System>, String)>>(
    iter: T,
) -> Option<SourceFile> {
    let (mut syscount, mut total) = (0, 0);
    let vec: Vec<_> = iter
        .map(|(b, sys, path)| {
            if sys.is_some() {
                syscount += 1;
            }
            total += 1;
            (b, sys, path)
        })
        .collect();
    Some(match (syscount, total, vec) {
        (0, 0, _) => return None,
        (0, 1, mut vec) => {
            let (_, _, path) = vec.pop()?;
            SourceFile::SourceWithNoSystem(path)
        }
        (1, 1, mut vec) => {
            let (_, sys, path) = vec.pop()?;
            SourceFile::SourceWithSystem(sys?, path)
        }
        (x, y, vec) if x == y => {
            if vec.first()?.0 {
                SourceFile::DynamicSourceWithDefaultSystem(
                    vec.first().as_ref()?.1.as_ref()?.clone(),
                    vec.into_iter()
                        .skip(1)
                        .filter_map(|(_, sys, path)| Some((sys?, path)))
                        .collect(),
                )
            } else {
                SourceFile::DynamicSourceWithDefaultPath(
                    vec.first()?.2.clone(),
                    vec.into_iter()
                        .skip(1)
                        .filter_map(|(_, sys, path)| Some((sys?, path)))
                        .collect(),
                )
            }
        }
        (x, y, vec) if y == x + 1 => SourceFile::DynamicSourceMap(
            vec.into_iter()
                .filter_map(|(_, sys, path)| Some((sys?, path)))
                .collect(),
        ),
        (_, _, _) => return None,
    })
}

impl SourceFile {
    pub fn insert_link(self, sys: &System, dest_string: &str) -> Result<Self> {
        let dest_string = dest_string.to_owned();
        let sys = sys.clone();
        Ok(match self {
            SourceFile::SourceWithNoSystem(path) => {
                let map = cascade! {
                    HashMap::new();
                    ..insert(sys, dest_string);
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
        self.clone().into_iter().any(|(_, _, x)| x == path)
    }

    pub fn resolve(&self, system: &Option<System>) -> Option<String> {
        let system = match system {
            None => match self {
                SourceFile::SourceWithNoSystem(path) => return Some(path.clone()),
                SourceFile::DynamicSourceWithDefaultPath(path, _) => return Some(path.clone()),
                _ => return None,
            },
            Some(x) => x,
        };
        match self {
            SourceFile::SourceWithNoSystem(path) => Some(path.clone()),
            SourceFile::DynamicSourceMap(system_map) => system_map.get(system).cloned(),
            SourceFile::DynamicSourceWithDefaultSystem(sys, system_map) => system_map
                .get(system)
                .or_else(|| system_map.get(sys))
                .cloned(),
            SourceFile::DynamicSourceWithDefaultPath(path, system_map) => {
                system_map.get(system).or(Some(path)).cloned()
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

    pub fn remove_link(self, search_path: &str) -> Option<SourceFile> {
        match self {
            SourceFile::SourceWithNoSystem(a) => {
                if search_path != a {
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
            SourceFile::SourceWithSystem(sys, path) => {
                if search_path != path {
                    Some(SourceFile::SourceWithSystem(sys, path))
                } else {
                    None
                }
            }
            SourceFile::DynamicSourceWithDefaultPath(path, map) => {
                if search_path == path {
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

    pub fn with_default(
        base_url: &Path,
        default: String,
        system_map: HashMap<System, String>,
    ) -> Result<SourceFile> {
        let new_map: HashMap<System, String> = system_map
            .into_iter()
            .map(move |(key, elem)| {
                check_path(&base_url.join(&elem))?;
                Ok((key, elem))
            })
            .try_collect::<_, _, anyhow::Error>()?;

        Ok(SourceFile::DynamicSourceWithDefaultSystem(
            System(default),
            new_map,
        ))
    }
}
