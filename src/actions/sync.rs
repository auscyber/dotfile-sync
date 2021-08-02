use crate::ProjectContext;
use anyhow::{Context, Result};
use futures::future::join_all;
use log::*;
use std::sync::Arc;
use tokio::fs;

pub async fn sync(ctx: ProjectContext) -> Result<()> {
    let ctx = Arc::new(ctx);
    let threads = ctx.project.links.clone().into_iter().map(|link| {
        let ctx = ctx.clone();
        //Creat async threads
        tokio::spawn(async move {
            async {
                let project_path = &ctx.project_config_path;
                let source = match link.src.resolve(&ctx.system) {
                    Some(d) => project_path.join(d),
                    None => return Ok(()),
                };
                debug!("project_path is {}", project_path.display());
                let destination = {
                    let mut temp_dest = link
                        .destination
                        .to_path_buf(ctx.project.variables.as_ref())?;
                    if temp_dest.is_dir()
                        && temp_dest.exists()
                        && !same_file::is_same_file(&temp_dest, &source)?
                    {
                        temp_dest.push(
                            source
                                .file_name()
                                .context(format!("Could not get file name for {}", link.name))?,
                        );
                    }
                    if temp_dest.exists() && same_file::is_same_file(&temp_dest, &source)? {
                        info!(r#""{}" already linked"#, source.display());
                        return Ok(());
                    } else if temp_dest.exists() {
                        error!("{} file already exists", source.display());
                        return Ok(());
                    }
                    temp_dest
                };
                fs::create_dir_all(
                    &destination
                        .parent()
                        .context("Could not get parent folder")?,
                )
                .await
                .context(format!(
                    "Failed creating folder hierchy for {}",
                    &destination.display()
                ))?;

                fs::symlink(source, destination).await?;
                Ok::<_, anyhow::Error>(())
            }
            .await
            .context(format!("Failed linking {}", &link.name))
        })
    });
    join_all(threads)
        .await
        .into_iter()
        .map(|x| x?)
        .collect::<Result<Vec<_>>>()?;
    Ok(())
}
