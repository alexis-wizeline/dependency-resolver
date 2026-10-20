use std::{collections::HashMap, result::Result};
use thiserror::Error;

pub struct Task {
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

pub struct DependencyGraph {
    tasks: HashMap<String, Task>,
}

impl DependencyGraph {
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
            .ok_or(GraphError::UnknowTask(task_id.to_string()))?;
        let already_dependecy = task.dependecies.iter().any(|id| id == dependency_id);
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
    fn test_dependency_grap_add_task() {
        let mut graph = DependencyGraph::new();
        assert_eq!(
            graph.tasks.len(),
            0,
            "expect the graph to be initialized with 0 tasks"
        );
        graph.add_task("A").expect("to Task 'A' to be added");
        assert_eq!(graph.tasks.len(), 1, "expect the graph to contain one task");
        let keys: Vec<String> = graph.tasks.into_keys().collect();
        assert_eq!(keys, vec!["A"], "expect key 'A' to be present");
    }

    #[test]
    fn test_dependency_grap_add_task_duplicated_task() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' a to be added");
        let duplicate_err = graph.add_task("A");
        assert!(
            duplicate_err.is_err(),
            "expect an error to be returned when trying to add same task"
        );
        assert_eq!(
            duplicate_err.err(),
            Some(GraphError::DuplicatedTask(String::from("A")))
        );
        let keys: Vec<String> = graph.tasks.into_keys().collect();
        assert_eq!(
            keys,
            vec!["A"],
            "excpect only a single element in the tasks for the graph"
        );
    }

    #[test]
    fn test_dependency_task_add_dependency() {
        let mut graph = DependencyGraph::new();
        graph
            .add_task("A")
            .expect("Expect for Task 'A' to be added");
        graph
            .add_task("B")
            .expect("Expect for Task 'B' to be added");
        graph
            .add_dependency("A", "B")
            .expect("for 'B' to be as dependency for 'A'");
        match graph.tasks.get("A") {
            Some(task) => {
                assert_eq!(
                    task.dependecies,
                    vec!["B"],
                    "to 'B' to be present in teh depencies array"
                );
            }
            None => {
                panic!("expect for Task 'A' to exists");
            }
        }
    }

    #[test]
    fn test_dependency_task_add_dependency_missing_dependency_task() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to Task 'A' to be added");
        let missing_depency_err = graph.add_dependency("A", "B");
        assert!(
            missing_depency_err.is_err(),
            "expect to faile because missing dependecy task"
        );
        assert_eq!(
            missing_depency_err.err(),
            Some(GraphError::UnknowTask(String::from("B")))
        );
    }

    #[test]
    fn test_dependency_task_add_dependency_missing_task() {
        let mut graph = DependencyGraph::new();
        graph.add_task("B").expect("to task 'B' to be Added");
        let missing_task_err = graph.add_dependency("A", "B");
        assert_eq!(
            missing_task_err.err(),
            Some(GraphError::UnknowTask(String::from("A")))
        );
    }

    #[test]
    fn test_dependency_task_add_dependency_duplicated_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to Task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be Added");
        graph
            .add_dependency("A", "B")
            .expect("for 'B' to be added as dependecy on 'A'");
        let duplicated_err = graph.add_dependency("A", "B");
        assert_eq!(
            duplicated_err.err(),
            Some(GraphError::DuplicatedDependency {
                dependency_id: String::from("B"),
                task_id: String::from("A")
            })
        );
    }
}
