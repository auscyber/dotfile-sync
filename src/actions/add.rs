use crate::{config::ProjectConfig, link::*, ProjectContext};
use anyhow::{bail, Context, Result};
use cascade::cascade;
use log::*;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn add(
    ctx: &ProjectContext,
    //File to copy
    original_location: String,
    //Location of where to place it in the project
    destination: Option<String>,
    name: Option<String>,
) -> Result<ProjectConfig> {
    let project_config = &ctx.project;
    let original_location_cleaned = PathBuf::from(crate::util::parse_vars(
        true,
        project_config.variables.as_ref(),
        &original_location,
    )?)
    .canonicalize()
    .context("file could not be located")?;
    let original_location = {
        let p = PathBuf::from(original_location);
        if same_file::is_same_file(&p, &original_location_cleaned)? {
            if !p.has_root() {
                std::env::current_dir()?.join(p)
            } else {
                p
            }
        } else {
            original_location_cleaned.clone()
        }
    }
    .to_string_lossy()
    .to_string();
    let output_dest: String = match destination.map(PathBuf::from) {
        Some(destination) => {
            let mut output = match destination.clone().strip_prefix(&ctx.project_config_path) {
                Ok(x) => x.to_path_buf(),
                _ => destination,
            };
            if ctx.project_config_path.join(&output).is_dir() {
                output = output.join(original_location_cleaned.file_name().context(format!(
                    "Could not get filename for {}",
                    original_location_cleaned.to_str().unwrap()
                ))?)
            }
            output.to_string_lossy().to_string()
        }

        None => original_location_cleaned
            .file_name()
            .map(|x| x.to_string_lossy().into())
            .context("Could not get file name")?,
    };

    if ctx
        .project_config_path
        .join(&output_dest)
        .canonicalize()
        .map(|x| x.exists())
        .unwrap_or(false)
        || project_config
            .links
            .iter()
            .any(|x| x.src.contains_path(&output_dest))
    {
        debug!(
            "path {}",
            ctx.project_config_path
                .join(&output_dest)
                .canonicalize()?
                .display()
        );
        bail!("Destination {} already exists", original_location);
    }
    let name = name.unwrap_or(
        original_location_cleaned
            .file_name()
            .map(|x| x.to_string_lossy().into())
            .context("Could not get file name")?,
    );
    let get_system = || ctx.system.to_owned().context("could not get system");
    let mut found = false;
    let mut completed_links = project_config
        .links
        .iter()
        .map(|link| {
            if link
                .destination
                .to_path_buf(project_config.variables.as_ref())
                .and_then(|x| Ok(x.canonicalize()? != original_location_cleaned))
                .unwrap_or(true)
            {
                return Ok(link.clone());
            }
            found = true;

            let mut link = link.clone();
            let sys = get_system()?;
            link.src = link.src.insert_link(&sys, &output_dest)?;
            Ok(link)
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    if !found {
        let source = match &ctx.args.system {
            Some(sys) => SourceFile::SourceWithSystem(sys.clone(), output_dest.to_owned()),
            None => SourceFile::SourceWithNoSystem(output_dest.to_owned()),
        };
        completed_links.push(Link::new(
            name.clone(),
            VariablePath::from_string(original_location),
            source,
        )?);
    };
    fs::create_dir_all(
        PathBuf::from(&output_dest)
            .parent()
            .context("Could not get parent folder")?,
    )
    .await?;
    let output_dest = ctx.project_config_path.join(output_dest);
    let final_project_config = cascade! {
        ctx.project.clone();
        ..links = completed_links;
    };
    move_link(&original_location_cleaned, &output_dest).await?;
    info!("Added {}", name);
    Ok(final_project_config)
}

async fn move_link(original_locaction_cleaned: &Path, output_dest: &Path) -> Result<()> {
    fs::copy(original_locaction_cleaned, output_dest).await?;
    if fs::metadata(original_locaction_cleaned).await?.is_dir() {
        fs::remove_dir_all(original_locaction_cleaned).await?;
    } else {
        fs::remove_file(original_locaction_cleaned).await?;
    }
    debug!(
        "loc = {} \n dest = {}",
        original_locaction_cleaned.display(),
        output_dest.display()
    );

    fs::symlink(output_dest, original_locaction_cleaned).await?;
    Ok(())
}
