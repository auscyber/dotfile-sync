use crate::goals::Goal;
use crate::ProjectContext;
use clap::Parser;
use snafu::Snafu;

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Snafu)]
enum GoalError {
    #[snafu(display("No goals for project"))]
    NoGoals,
    #[snafu(display("File {} not in project",path))]
    NotInProject {
        path: PathBuf
    }
}

#[derive(Parser, Clone)]
pub enum GoalSubCommand {
    List,
    Add { name: String, depends: Vec<String> },
    AddFile { goal: String, files: Vec<PathBuf> },
}

pub async fn goals(
    ctx: &ProjectContext,
    command: GoalSubCommand,
) -> Result<crate::config::ProjectConfig,GoalError> {
    let mut project_config = ctx.project.clone();
    use GoalSubCommand::*;
    match command {
        List => match ctx.project.goals {
            Some(ref goals) => {
                println!("Goals: \n");
                for (name, goal) in goals {
                    print!("Name: {} \n {}", name, goal);
                }
            }
            None => return Err(GoalError::NoGoals),
        },
        AddFile { goal, files } => {
            for file in files {
                snafu::ensure!(
                    ctx.in_project(
                        &crate::config::ProjectConfig::remove_start(
                            &ctx.project_config_path,
                            &file
                        )
                        .context("does not start with config_path")?
                    )?,
                    "File not in project"
                );
                project_config
                    .goals
                    .as_mut()
                    .context("No Goals for project".to_string())?
                    .get_mut(&goal)
                    .context(format!("Could not find goal {}", goal))?
                    .links
                    .push(file.to_str().unwrap().to_string());
            }
        }
        Add { name, depends } => {
            let _ = project_config
                .goals
                .get_or_insert_with(HashMap::new)
                .insert(name, Goal::new(depends));
        }
    }
    Ok(project_config)
}
