#[warn(deprecated)]
use crate::{config::*, file_actions, link::*};
use anyhow::{bail, Context, Result};
use log::*;
use std::{fs, path::PathBuf};

pub fn sync(project: ProjectConfig, path: PathBuf, system: Option<System>) -> Result<()> {
    for link in project.links {
        let destination = match link.destination.resolve(&system) {
            Some(d) => path.join(d),
            None => continue,
        };
        debug!("project_path is {}", path.join(&destination).display());
        info!("Linking {}", link.name);
        let src = link.src.to_path_buf(None)?;
        crate::util::create_folders(&src).context(format!("Failed creating folder hierchy for {}",&src.display()))?;
        if let Err(err) = fs::soft_link(path.join(destination), src)
            .context(format!("Failed linking {}", &link.name))
        {
            error!("{}", err);
            err.chain()
                .skip(1)
                .for_each(|cause| error!("\treason : {}", cause));
        };
    }
    Ok(())
}

pub fn manage(
    sysconfig: SystemConfig,
    project: ProjectConfig,
    project_path: PathBuf,
) -> Result<SystemConfig> {
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
    project_file_loc: Option<String>,
    system: Option<System>,
    project_path: PathBuf,
) -> Result<ProjectConfig> {
    let mut project = project.clone();
    let project_file_loc = project_file_loc
        .or_else(|| local_location.to_str().map(|x| x.to_string()))
        .unwrap();
    let destination_ = system
        .map(|ref sys| {
            Ok({
                project
                    .links
                    .iter()
                    .find(|link| {
                        let buf = link.src.clone().to_path_buf(None);
                        if let Ok(a) = buf {
                            a == local_location
                        } else {
                            false
                        }
                    })
                    .map(|link| {
                        Ok(match link.destination.clone() {
                            Destination::DefaultDest(path) => {
                                let a = project
                                    .default
                                    .clone()
                                    .context("No default system to wrap pre-existing file")?;
                                Destination::DynamicDestination(
                                    vec![(a, path), (sys.clone(), project_file_loc.clone())]
                                        .into_iter()
                                        .collect(),
                                )
                            }
                            Destination::SystemDest(system, path) => {
                                Destination::DynamicDestination(
                                    vec![(system, path), (sys.clone(), project_file_loc.clone())]
                                        .into_iter()
                                        .collect(),
                                )
                            }
                            Destination::DynamicDestination(mut system_map) => {
                                system_map.insert(sys.clone(), project_file_loc.clone());
                                Destination::DynamicDestination(system_map)
                            }
                            Destination::DynamicDestinationWithDefault(
                                default_system,
                                mut system_map,
                            ) => {
                                system_map.insert(sys.clone(), project_file_loc.clone());
                                Destination::DynamicDestinationWithDefault(
                                    default_system,
                                    system_map,
                                )
                            }
                        })
                    })
                    .unwrap_or(Ok::<Destination, anyhow::Error>(Destination::SystemDest(
                        sys.clone(),
                        project_file_loc.clone(),
                    )))?
            })
        })
        .unwrap_or(Ok::<Destination, anyhow::Error>(Destination::DefaultDest(
            project_file_loc.clone(),
        )))?;
    crate::util::create_folders(project_path.join(&project_file_loc))?;
    let link = Link::new(
        name,
        VariablePath::from_path(&local_location)?,
        destination_,
    )?;

    file_actions::mv_link(&local_location, &project_path.join(&project_file_loc))
        .context("Failure linking")?;

    project.links.push(link);
    Ok(project)
}

pub fn prune(proj_path: PathBuf, project: ProjectConfig) -> ProjectConfig {
    todo!();
    let mut clone = project.clone();
    clone.links = project
        .links
        .iter()
        .filter_map(|x| {
            let mut x = x.clone();
            if !x.src.clone().to_path_buf(None).ok()?.exists() {
                return None;
            } else {
                return Some(x);
            }
            // let mut x = x.clone();
            // x.destination = match x.destination {
            //     Destination::DefaultDest(a) => proj_path.join(a.clone()).canonicalize().map(|_| Destination::DefaultDest(a)).ok(),
            //     Destination::DynamicDestination(a) => {
            //         let map: HashMap<System, String> = a
            //             .iter()
            //             .filter_map(|(a, x)| {
            //                 proj_path.join(x).canonicalize().ok();
            //                 Some((a.clone(), x.clone()))
            //             })
            //             .collect();
            //         if map.len() == 0 {
            //             None
            //         } else {
            //             Some(Destination::DynamicDestination(map))
            //         }
            //     }
            //     Destination::SystemDest(sys, a) => {
            //         proj_path.join(a.clone()).canonicalize().map(|_| Destination::SystemDest(sys, a)).ok()
            //     }
            //     Destination::DynamicDestinationWithDefault(def, a) => {
            //         let map: HashMap<System, String> = a
            //             .iter()
            //             .filter_map(|(a, x)| {
            //                 proj_path.join(x).canonicalize().ok();
            //                 Some((a.clone(), x.clone()))
            //             })
            //             .collect();
            //         if map.contains_key(&def) {
            //             return None;
            //         }
            //         if map.is_empty() {
            //             None
            //         } else {
            //             Some(Destination::DynamicDestinationWithDefault(def, a))
            //         }
            //     }
            // }?;
            //            Some(x)
        })
        .collect::<Vec<Link>>();
    clone
}

pub fn revert(
    path: PathBuf,
    project: ProjectConfig,
    proj_path: &PathBuf,
    system: Option<System>,
) -> Result<ProjectConfig> {
    let path = path.canonicalize()?;
    let proj_path = proj_path
        .canonicalize()
        .context("Could not canonicalize project path")?;
    if let None = ProjectConfig::remove_start(&proj_path, &path) {
        bail!("File is not a link inside valid directory")
    }
    let links = project
        .links
        .iter()
        .map(|link| {
            let src_path = link.src.clone().to_path_buf(None)?;
            let mut link = link.clone();
            let dest_str = link
                .destination
                .clone()
                .resolve(&system)
                .context("Could not resolve system")?;
            let dest = proj_path
                .join(PathBuf::from(dest_str.clone()))
                .canonicalize()?;
            if dest == path {
                if path.is_file() {
                    fs::remove_file(&src_path)?;
                } else {
                    fs::remove_dir_all(&src_path)?;
                }
                file_actions::copy_path(dest, &src_path).context("Error copying file")?;
                link.destination = match link.destination.clone().remove_link(&dest_str) {
                    Some(x) => x,
                    None => {
                        if path.is_file() {
                            fs::remove_file(path.clone())?;
                        } else {
                            fs::remove_dir_all(path.clone())?;
                        }
                        info!("Copied file ");
                        return Ok(None);
                    }
                };
            }
            Ok(Some(link.clone()))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|x| x)
        .collect();
    let mut project2 = project.clone();
    project2.links = links;
    Ok(project2)
}
