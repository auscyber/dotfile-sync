use crate::{config::*, link::*};
use log::*;

use anyhow::*;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn revert(ctx: &crate::ProjectContext, path: &Path) -> Result<ProjectConfig> {
    debug!("path is {}", path.display());
    let ac_path = path.canonicalize().context("could not find file")?;
    let mut new_project = ctx.project.clone();
    let mut dest_path: Option<PathBuf> = None;
    let new_links = new_project
        .links
        .iter()
        .filter_map(|link| {
            let mut new_link = link.clone();
            let link_dest = link
                .destination
                .to_path_buf(new_project.variables.as_ref())
                .ok();

            debug!("link_dest = {:?} {:?}", link_dest, ac_path);
            new_link.src = convert_iter_to_source(link.src.clone().into_iter().filter(|x| {
                println!("src = {}", ctx.project_config_path.join(&x.2).display());
                if same_file::is_same_file(ctx.project_config_path.join(&x.2), &ac_path)
                    .unwrap_or(false)
                {
                    debug!("found it");
                    dest_path = link_dest.clone();
                    false
                } else {
                    true
                }
            }))?;
            Some(new_link)
        })
        .collect();
    let dest = dest_path.context("could not find path in links")?;
    println!("dest is {}", dest.display());
    fs::remove_file(&dest).await?;
    fs::copy(&ac_path, dest).await?;
    if ac_path.is_dir() {
        fs::remove_file(&ac_path).await?;
    } else {
        fs::remove_dir_all(&ac_path).await?;
    }
    new_project.links = new_links;
    Ok(new_project)
}
