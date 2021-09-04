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
    #[serde(flatten)]
    pub src: SourceFile,
}

impl Link {
    pub fn new(name: String, src: VariablePath, destination: SourceFile) -> Link {
        Link {
            name,
            destination: src,
            src: destination,
        }
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
#[serde(transparent)]
pub struct SourceFileUrl(String);

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
#[serde(untagged)]
pub enum SourceFile {
    Source {
        system: Option<System>,
        src: String,
    },
    DynamicSource {
        default_path: Option<String>,
        default_system: Option<System>,
        source_map: HashMap<System, String>,
    },
}

//impl<'de> Deserialize<'de> for SourceFile {
//    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//    where
//        D: serde::Deserializer<'de>,
//    {
//        struct
//    }
//}
//impl<'se> Serialize<'se> for SourceFile {}

impl IntoIterator for SourceFile {
    type Item = (bool, Option<System>, String);
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;
    fn into_iter(self) -> Self::IntoIter {
        use SourceFile::*;
        match self {
            Source { src: path, system } => Box::new(Some((false, system, path)).into_iter()),
            DynamicSource {
                default_path,
                source_map: map,
                default_system,
            } => Box::new(default_path.map(|s| (false, None, s)).into_iter().chain(
                map.into_iter().map(move |(sys, path)| {
                    (
                        default_system
                            .as_ref()
                            .map(|system| system == &sys)
                            .unwrap_or(false),
                        Some(sys),
                        path,
                    )
                }),
            )),
        }
    }
}
impl fmt::Display for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceFile::Source { system, src: path } => {
                write!(f, "\t")?;
                system
                    .as_ref()
                    .map_or(Ok(()), |sys| write!(f, "System: {} ", sys))?;
                writeln!(f, "Path: {}", path)
            }
            SourceFile::DynamicSource {
                default_system,
                default_path,
                source_map: map,
            } => {
                write!(f, "\t")?;
                default_path
                    .as_ref()
                    .map_or(Ok(()), |p| write!(f, "Default Path: {} ", p))?;
                default_system
                    .as_ref()
                    .map_or(Ok(()), |s| write!(f, "Default System: {}", s))?;
                if default_path.is_some() || default_system.is_some() {
                    writeln!(f)?;
                }
                map.iter().try_for_each(|(system, path)| {
                    writeln!(f, "\tSystem: {}, Path: {}", system, path)
                })
            }
        }
    }
}

pub fn convert_iter_to_source<T: Iterator<Item = (bool, Option<System>, String)>>(
    iter: T,
) -> Option<SourceFile> {
    let mut total = 0;
    let vec: Vec<_> = iter
        .map(|(b, sys, path)| {
            total += 1;
            (b, sys, path)
        })
        .collect();
    match (total, vec) {
        (0, _) => None,
        (1, mut vec) => {
            let (_, system, path) = vec.pop()?;
            Some(SourceFile::Source { system, src: path })
        }
        (_, vec) => Some(SourceFile::DynamicSource {
            default_path: vec.iter().find_map(|x| {
                if x.1.is_none() {
                    Some(x.2.clone())
                } else {
                    None
                }
            }),
            default_system: vec.iter().find_map(|x| {
                if x.0 {
                    Some(x.1.as_ref()?.clone())
                } else {
                    None
                }
            }),
            source_map: vec
                .iter()
                .filter_map(|x| {
                    if !x.0 {
                        Some((x.1.as_ref()?.clone(), x.2.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
        }),
    }
}

impl SourceFile {
    pub fn insert_link(self, sys: &System, dest_string: &str) -> Result<Self> {
        let dest_string = dest_string.to_owned();
        let sys = sys.clone();
        Ok(match self {
            SourceFile::Source { system, src: path } => {
                if !system.as_ref().map_or(false, |x| x == &sys) {
                    let map = cascade! {
                        HashMap::new();
                        ..insert(sys, dest_string);
                    };
                    SourceFile::DynamicSource {
                        default_system: system,
                        default_path: Some(path),
                        source_map: map,
                    }
                } else {
                    bail!(
                        r#"System "{:?}" already defined for output file "{}" "#,
                        sys,
                        dest_string
                    );
                }
            }
            SourceFile::DynamicSource {
                default_path,
                default_system,
                source_map: mut map,
            } => {
                if map.contains_key(&sys) || default_system.as_ref().map_or(false, |x| x == &sys) {
                    bail!(
                        r#"System "{}" already defined for output file "{}" "#,
                        sys,
                        dest_string
                    );
                }
                map.insert(sys, dest_string);
                SourceFile::DynamicSource {
                    default_path,
                    default_system,
                    source_map: map,
                }
            }
        })
    }
    pub fn contains_path(&self, path: &str) -> bool {
        self.clone().into_iter().any(|(_, _, x)| x == path)
    }

    pub fn resolve(&self, system: &Option<System>) -> Option<String> {
        let system = match system {
            None => match self {
                SourceFile::Source { src: path, .. } => return Some(path.clone()),
                SourceFile::DynamicSource {
                    default_path: Some(default_path),
                    ..
                } => return Some(default_path.clone()),
                _ => return None,
            },
            Some(x) => x,
        };
        match self {
            SourceFile::Source {
                src: path,
                system: sys,
            } => {
                if sys.as_ref().map_or(true, |x| x == system) {
                    Some(path.clone())
                } else {
                    None
                }
            }
            SourceFile::DynamicSource {
                default_path,
                default_system,
                source_map: map,
            } => map
                .get(system)
                .or_else(|| map.get(default_system.as_ref()?))
                .or_else(|| default_path.as_ref())
                .cloned(),
        }
    }

    pub fn remove_link(self, search_path: &str) -> Option<SourceFile> {
        match self {
            SourceFile::Source { src: path, system } => {
                if search_path != path {
                    Some(SourceFile::Source { src: path, system })
                } else {
                    None
                }
            }
            SourceFile::DynamicSource {
                default_path,
                source_map: map,
                default_system,
            } => {
                if default_path.as_ref().map_or(false, |x| x == search_path) {
                    return Some(SourceFile::DynamicSource {
                        default_path: None,
                        source_map: map,
                        default_system,
                    });
                }
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
                    Some(SourceFile::DynamicSource {
                        default_path,
                        default_system,
                        source_map: map,
                    })
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

        Ok(SourceFile::DynamicSource {
            default_path: None,
            default_system: Some(System(default)),
            source_map: new_map,
        })
    }
}
