use crate::{config::*, file_actions, link::*};
use anyhow::{bail, Context, Result};
use std::{collections::HashMap, fs, path::PathBuf};
pub fn sync(project: ProjectConfig, path: PathBuf, system: System) -> Result<()> {
    project
        .links
        .iter()
        .map(|x| {
            {
                let destination = path.join(match x.destination.clone() {
                    Destination::DefaultDest(y) => y,
                    Destination::DynamicDestination(y) => {
                        if let Some(a) = y.get(&system) {
                            a.clone()
                        } else {
                            return Ok(());
                        }
                    }
                    Destination::SystemDest(y, a) => {
                        if y == system {
                            a.clone()
                        } else {
                            return Ok(());
                        }
                    }
                    Destination::DynamicDestinationWithDefault(a, map) => {
                        if let Some(b) = map.get(&system).or_else(|| map.get(&a)) {
                            b.clone()
                        } else {
                            return Ok(());
                        }
                    }
                });
                println!("Linking {}", x.name);
                fs::soft_link(destination, &x.src).context(format!("Failed linking {}", &x.name))?;
                let res: Result<()> = Ok(());
                res
            }
            .with_context(|| format!("Failed on {}", x.name))
        })
        .collect::<Result<_>>()
}

pub fn manage(sysconfig: SystemConfig, project: ProjectConfig, project_path: PathBuf) -> Result<SystemConfig> {
    let mut sysconfig = sysconfig.clone();
    if !project_path.exists() {
        bail!("Project path does not exist");
    }
    sysconfig.add_project(project.name, project_path);
    Ok(sysconfig)
}

pub fn add(
    project: ProjectConfig,
    name: String,
    local_location: PathBuf,
    project_file_loc: String,
    system: Option<System>,
    project_path: PathBuf,
) -> Result<ProjectConfig> {
    let mut project = project.clone();
    let destination_ = match system {
        Some(sys) => match project.links.iter().find(|x| x.src == local_location).map(|x| x.destination.clone()) {
            Some(Destination::DefaultDest(path)) => {
                let a = project.default.clone().context("No default system to wrap pre-existing file")?;
                Destination::DynamicDestination(vec![(a, path), (sys, project_file_loc.clone())].into_iter().collect())
            }
            Some(Destination::SystemDest(a, path)) => {
                Destination::DynamicDestination(vec![(a, path), (sys, project_file_loc.clone())].into_iter().collect())
            }
            Some(Destination::DynamicDestination(mut a)) => {
                a.insert(sys, project_file_loc.clone());
                Destination::DynamicDestination(a)
            }
            Some(Destination::DynamicDestinationWithDefault(def, mut a)) => {
                a.insert(sys, project_file_loc.clone());
                Destination::DynamicDestinationWithDefault(def, a)
            }

            None => Destination::DefaultDest(project_file_loc.clone()),
        },
        None => Destination::DefaultDest(project_file_loc.clone()),
    };
    let link = Link::new(name, local_location.canonicalize()?.to_str().unwrap().into(), destination_)?;
    file_actions::mv_link(&local_location, &project_path.join(&project_file_loc)).context("Failure linking")?;

    project.links.push(link);
    Ok(project)
}

pub fn prune(proj_path: PathBuf, project: ProjectConfig) -> ProjectConfig {
    let mut clone = project.clone();
    clone.links = project
        .links
        .iter()
        .filter_map(|x| {
            let mut x = x.clone();
            x.destination = match x.destination {
                Destination::DefaultDest(a) => proj_path.join(a.clone()).canonicalize().map(|_| Destination::DefaultDest(a)).ok(),
                Destination::DynamicDestination(a) => {
                    let map: HashMap<System, String> = a
                        .iter()
                        .filter_map(|(a, x)| {
                            proj_path.join(x).canonicalize().ok();
                            Some((a.clone(), x.clone()))
                        })
                        .collect();
                    if map.len() == 0 {
                        None
                    } else {
                        Some(Destination::DynamicDestination(map))
                    }
                }
                Destination::SystemDest(sys, a) => {
                    proj_path.join(a.clone()).canonicalize().map(|_| Destination::SystemDest(sys, a)).ok()
                }
                Destination::DynamicDestinationWithDefault(def, a) => {
                    let map: HashMap<System, String> = a
                        .iter()
                        .filter_map(|(a, x)| {
                            proj_path.join(x).canonicalize().ok();
                            Some((a.clone(), x.clone()))
                        })
                        .collect();
                    map.get(&def)?;
                    if map.len() == 0 {
                        None
                    } else {
                        Some(Destination::DynamicDestinationWithDefault(def, a))
                    }
                }
            }?;
            Some(x)
        })
        .collect::<Vec<Link>>();
    clone
}

pub fn revert(path: PathBuf, project: ProjectConfig, proj_path: &PathBuf) -> Result<ProjectConfig> {
    let links = project
        .links
        .iter()
        .filter_map(|x| {
            let mut x = x.clone();
            x.destination = x.destination.remove_link(&ProjectConfig::remove_start(&proj_path, &path)?)?;
            Some(x)
        })
        .collect();
    let mut project2 = project.clone();
    project2.links = links;
    Ok(project2)
}
