use crate::{config::*, file_actions, link::*, ProjectContext};
use anyhow::{bail, Context, Result};
use cascade::cascade;
use log::*;
use std::collections::HashMap;
use std::path::PathBuf;
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
        Some(ctx.system_config.global_variables.clone()),
        &original_location,
    )?)
    .canonicalize()?;

    let output: String = match destination.map(|x| PathBuf::from(x)) {
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

    if ctx.project_config_path.join(&output).canonicalize().is_ok() {
        bail!("Destination {} already exists", original_location);
    }
    if project_config
        .links
        .iter()
        .find(|x| x.src.contains_path(&output))
        .is_some()
    {
        bail!("Destination {} already exists", original_location);
    }
    let get_system = || ctx.system.to_owned().context("could not get system");
    let mut found = false;
    let mut completed_links = project_config
        .links
        .iter()
        .map(|x| {
            if x.src
                .resolve(&ctx.system)
                .map(|x| x == output)
                .unwrap_or(false)
            {
                return Ok(x.clone());
            }
            found = true;

            let mut link = x.clone();
            let sys = get_system()?;
            link.src = link.src.insert_link(&sys, &output)?;
            Ok(link)
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    if !found {
        let source = match &ctx.args.system {
            Some(sys) => SourceFile::SourceWithSystem(sys.clone(), output.to_owned()),
            None => SourceFile::SourceWithNoSystem(output.to_owned()),
        };
        completed_links.push(Link::new(
            name.unwrap_or(
                original_location_cleaned
                    .file_name()
                    .map(|x| x.to_string_lossy().into())
                    .context("Could not get file name")?,
            ),
            VariablePath::from_string(original_location),
            source,
        )?);
    };
    fs::copy(original_location_cleaned, output).await?;

    todo!();
}
