use anyhow::Result;
use std::collections::HashMap;
use thiserror::Error;

use crate::graph::GraphError::UnknowTask;

pub struct Task {
    id: String,
    dependecies: Vec<String>,
}

#[derive(PartialEq, Eq, Debug, Error)]
pub enum GraphError {
    #[error("Task {0} already exist")]
    DuplicatedTask(String),
    #[error("Task {0} does not exist")]
    UnknowTask(String),
    #[error("Task {dependency_id} is already a depdency of {task_id}")]
    DuplicatedDependency {
        dependency_id: String,
        task_id: String,
    },
}

pub struct DependecyGraph {
    tasks: HashMap<String, Task>,
}

impl DependecyGraph {
    pub fn new() -> Self {
        return Self {
            tasks: HashMap::new(),
        };
    }

    pub fn add_task(&mut self, id: &str) -> Result<(), GraphError> {
        let exist: bool = self.tasks.contains_key(id);
        if exist {
            return Err(GraphError::DuplicatedTask(id.to_string()));
        }

        let key = id.to_string();
        self.tasks.insert(
            key,
            Task {
                id: id.to_string(),
                dependecies: Vec::new(),
            },
        );

        Ok(())
    }

    pub fn add_dependency(&mut self, task_id: &str, dependency_id: &str) -> Result<(), GraphError> {
        let dependecy_exist = self.tasks.contains_key(dependency_id);
        if !dependecy_exist {
            return Err(GraphError::UnknowTask(dependency_id.to_string()));
        }
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or(UnknowTask(task_id.to_string()))?;
        let already_dependecy = task.dependecies.contains(&dependency_id.to_string());
        if already_dependecy {
            return Err(GraphError::DuplicatedDependency {
                dependency_id: dependency_id.to_string(),
                task_id: task_id.to_string(),
            });
        }

        task.dependecies.push(dependency_id.to_owned());
        Ok(())
    }
}

#[cfg(test)]
mod grap_test {
    use super::*;

    #[test]
    fn test_dependecy_graph() {
        let mut graph = DependecyGraph::new();
        let res = graph.add_dependency("A", "B");
        assert!(res.is_err(), "expect error for unkown dependency_id");
        assert_eq!(res.err(), Some(GraphError::UnknowTask(String::from("B"))));
        let res = graph.add_task("B");
        res.expect("to task 'B' to be added");
        let duplicated_task_res = graph.add_task("B");
        assert!(
            duplicated_task_res.is_err(),
            "expect to error out by inserting a duplicated task"
        );
        assert_eq!(
            duplicated_task_res.err(),
            Some(GraphError::DuplicatedTask(String::from("B")))
        );
        let unknow_task_res = graph.add_dependency("A", "B");
        assert!(
            unknow_task_res.is_err(),
            "excpect to fail because unknown task_id A"
        );
        assert_eq!(
            unknow_task_res.err(),
            Some(GraphError::UnknowTask(String::from("A")))
        );
        graph.add_task("A").expect("to Task 'A' to be added");
        graph
            .add_dependency("A", "B")
            .expect("to Task 'B' to be added as depedency to Task 'A'");
        let duplicated_dependency_err = graph.add_dependency("A", "B");
        assert!(
            duplicated_dependency_err.is_err(),
            "expect to erro out for adding an existing dependecy"
        );
        assert_eq!(
            duplicated_dependency_err.err(),
            Some(GraphError::DuplicatedDependency {
                dependency_id: String::from("B"),
                task_id: String::from("A")
            })
        )
    }
}
