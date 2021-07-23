#[warn(deprecated)]
use crate::{config::*, file_actions, link::*};
use anyhow::{bail, Context, Result};
use log::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

mod add;
mod prune;
mod sync;

pub use add::add;
pub use prune::prune;
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

pub fn revert(
    path: PathBuf,
    project: ProjectConfig,
    proj_path: &Path,
    system: Option<System>,
) -> Result<ProjectConfig> {
    let path = path.canonicalize()?;
    let proj_path = proj_path
        .canonicalize()
        .context("Could not canonicalize project path")?;
    if ProjectConfig::remove_start(&proj_path, &path).is_none() {
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
            Ok(Some(link))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let mut project2 = project;
    project2.links = links;
    Ok(project2)
}
