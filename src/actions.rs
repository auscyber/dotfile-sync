use crate::{config::*, file_actions, link::*};
use anyhow::{bail, Context, Result};
use std::{collections::HashMap, fs, path::PathBuf};

pub fn sync(project: ProjectConfig, path: PathBuf, system: System) -> Result<()> {
    for link in project.links {
        let destination = match link.destination.resolve(&system) {
            Some(d) => path.join(d),
            None => continue,
        };
        println!("Linking {}", link.name);
        fs::soft_link(destination, &link.src).context(format!("Failed linking {}", &link.name))?;
    }
    Ok(())
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
        Some(sys) => match project.links.iter().find(|link| link.src == local_location).map(|link| link.destination.clone()) {
            Some(Destination::DefaultDest(path)) => {
                let a = project.default.clone().context("No default system to wrap pre-existing file")?;
                Destination::DynamicDestination(vec![(a, path), (sys, project_file_loc.clone())].into_iter().collect())
            }
            Some(Destination::SystemDest(system, path)) => {
                Destination::DynamicDestination(vec![(system, path), (sys, project_file_loc.clone())].into_iter().collect())
            }
            Some(Destination::DynamicDestination(mut system_map)) => {
                system_map.insert(sys, project_file_loc.clone());
                Destination::DynamicDestination(system_map)
            }
            Some(Destination::DynamicDestinationWithDefault(default_system, mut system_map)) => {
                system_map.insert(sys, project_file_loc.clone());
                Destination::DynamicDestinationWithDefault(default_system, system_map)
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
                    if map.contains_key(&def) {
                        return None;
                    }
                    if map.is_empty() {
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
