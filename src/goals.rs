use crate::link::{Link, VariablePath};
use anyhow::{Context, Result};
use itertools::Itertools;
use petgraph::graph::DiGraph;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Goal {
    pub enabled: bool,
    pub name: String,
    pub links: Vec<String>,
    pub required_goals: Option<Vec<String>>,
}

impl Goal {
    pub fn to_assoc(&self, all_goals: &HashMap<String, &Goal>) -> Result<Vec<(String, String)>> {
        match self.required_goals {
            Some(ref goals) => Ok(goals
                .iter()
                .map(|goal_name| {
                    let goal = all_goals.get(goal_name).context(format!(
                        r#"Could not find goal by the name of "{}" "#,
                        goal_name
                    ))?;
                    let mut result = goal.to_assoc(&all_goals)?;
                    result.push((self.name.clone(), goal_name.clone()));
                    Ok(result)
                })
                .try_collect::<_, Vec<_>, anyhow::Error>()?
                .concat()),
            None => Ok(Vec::new()),
        }
    }

    pub fn to_deps(&self, goal_list: Vec<Goal>) -> Result<Vec<Goal>> {
        let goals = match self.required_goals {
            Some(ref x) => x,
            None => return Ok(Default::default()),
        };
        if goals.contains(&self.name) {
            return Err(anyhow::Error::msg("Goal contains itself"));
        }
        let goal_list_iter = goal_list.iter().map(|x| (x.name.clone(), x)).collect();

        let mut graph = DiGraph::<&str, ()>::new();
        let assoc = self.to_assoc(&goal_list_iter)?;
        let used_goals = assoc
            .iter()
            .dedup_by(|a, b| a.0 == b.0)
            .map(|x| (x.0.as_str(), graph.add_node(x.0.as_str())))
            .collect::<HashMap<_, petgraph::graph::NodeIndex<_>>>();
        graph.extend_with_edges(assoc.iter().filter_map(|(a, b)| {
            Some((
                used_goals.get(a.as_str())?.clone(),
                used_goals.get(b.as_str())?.clone(),
            ))
        }));
        println!("{:?}", petgraph::dot::Dot::new(&graph));
        todo!();
    }

    fn get_links(&self, ctx: &crate::ProjectContext) -> Result<Vec<Link>> {
        let project = &ctx.project;
        todo!();
    }
}
