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
                let destination = match link.src.resolve(&ctx.system) {
                    Some(d) => project_path.join(d),
                    None => return Ok(()),
                };
                debug!("project_path is {}", project_path.display());
                let src = link.destination.to_path_buf(None)?;
                fs::create_dir_all(&src.parent().context("Could not get parent folder")?)
                    .await
                    .context(format!(
                        "Failed creating folder hierchy for {}",
                        &src.display()
                    ))?;
                if src.exists() && src.canonicalize()? == destination.canonicalize()? {
                    info!(r#""{}" already linked"#, destination.display());
                    return Ok(());
                }

                fs::symlink(destination, src).await?;
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
