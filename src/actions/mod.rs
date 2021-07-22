#[warn(deprecated)]
use crate::{config::*, file_actions, link::*};
use anyhow::{bail, Context, Result};
use log::*;
use std::{fs, path::PathBuf};

mod add;
mod sync;

pub use add::add;
pub use sync::sync;

pub fn manage(ctx: &super::ProjectContext, make_default: bool) -> Result<SystemConfig> {
    let mut sysconfig = ctx.system_config.clone();
    if !ctx.project_config_path.exists() {
        bail!("Project path does not exist");
    }
    sysconfig.add_project(ctx.project.name.clone(), ctx.project_config_path.clone());
    if make_default {
        sysconfig.default = Some(ctx.project_config_path.clone());
        info!("Set as default");
    }

    Ok(sysconfig)
}

pub fn prune(proj_path: PathBuf, project: ProjectConfig) -> ProjectConfig {
    todo!();
    let mut clone = project.clone();
    clone.links = project
        .links
        .iter()
        .filter_map(|x| {
            let mut x = x.clone();
            if !x.destination.clone().to_path_buf(None).ok()?.exists() {
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
            let src_path = link.destination.clone().to_path_buf(None)?;
            let mut link = link.clone();
            let dest_str = link
                .src
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
                link.src = match link.src.clone().remove_link(&dest_str) {
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
