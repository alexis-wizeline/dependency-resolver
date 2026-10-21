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
    UnknownTask(String),
    #[error("Task {dependency_id} is already a depdency of {task_id}")]
    DuplicatedDependency {
        dependency_id: String,
        task_id: String,
    },
    #[error("A cycle dependecy found with {0}")]
    DependencyCylce(String),
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
        if self.tasks.contains_key(id) {
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
        if !self.tasks.contains_key(dependency_id) {
            return Err(GraphError::UnknownTask(dependency_id.to_string()));
        }
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or(GraphError::UnknownTask(task_id.to_string()))?;
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

    pub fn resolve(&self, task_id: &str) -> Result<Vec<String>, GraphError> {
        let mut result: Vec<String> = Vec::new();
        let mut pipeline: HashMap<String, StatedTask> = HashMap::new();

        self.visit_task(task_id, &mut result, &mut pipeline)?;

        Ok(result)
    }

    fn visit_task(
        &self,
        task_id: &str,
        result: &mut Vec<String>,
        pipeline: &mut HashMap<String, StatedTask>,
    ) -> Result<(), GraphError> {
        // 1.- task exist
        let task = self
            .tasks
            .get(task_id)
            .ok_or(GraphError::UnknownTask(task_id.to_string()))?;

        // 2.- is visiting?
        if pipeline
            .iter()
            .any(|(id, stated)| id == task_id && stated.is_visiting())
        {
            return Err(GraphError::DependencyCylce(task_id.to_string()));
        }

        // 3.- is visited?
        if pipeline
            .iter()
            .any(|(id, state)| id == task_id && state.is_visited())
        {
            return Ok(());
        }

        // 4.- add into the pipeline
        pipeline.insert(task_id.to_string(), StatedTask::new());

        // 5.- traverse dependencies.
        let dependencies = &task.dependecies;
        for dependency in dependencies {
            self.visit_task(dependency, result, pipeline)?;
        }

        // 6.- visited
        pipeline
            .entry(task_id.to_string())
            .and_modify(|stated| stated.visited());
        result.push(task_id.to_string());

        Ok(())
    }
}

#[derive(PartialEq, Debug)]
enum TraverseState {
    Visiting,
    Visited,
}

#[derive(Debug)]
struct StatedTask {
    state: TraverseState,
}

impl StatedTask {
    fn new() -> Self {
        Self {
            state: TraverseState::Visiting,
        }
    }

    fn is_visiting(&self) -> bool {
        self.state == TraverseState::Visiting
    }

    fn is_visited(&self) -> bool {
        self.state == TraverseState::Visited
    }

    fn visited(&mut self) {
        self.state = TraverseState::Visited
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
            Some(GraphError::UnknownTask(String::from("B")))
        );
    }

    #[test]
    fn test_dependency_task_add_dependency_missing_task() {
        let mut graph = DependencyGraph::new();
        graph.add_task("B").expect("to task 'B' to be Added");
        let missing_task_err = graph.add_dependency("A", "B");
        assert_eq!(
            missing_task_err.err(),
            Some(GraphError::UnknownTask(String::from("A")))
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
        assert_eq!(
            graph.tasks.get("A").expect("'A' to exist").dependecies,
            vec!["B"]
        );
        let duplicated_err = graph.add_dependency("A", "B");
        assert_eq!(
            duplicated_err.err(),
            Some(GraphError::DuplicatedDependency {
                dependency_id: String::from("B"),
                task_id: String::from("A")
            })
        );
        assert_eq!(
            graph.tasks.get("A").expect("'A' to exist").dependecies,
            vec!["B"]
        );
    }

    #[test]
    fn test_stated_task_visiting_when_created() {
        assert!(
            StatedTask::new().is_visiting(),
            "expect the task to be visiting when created"
        )
    }

    #[test]
    fn test_stated_task_visited_when_transition_happens() {
        let mut task = StatedTask::new();
        task.visited();
        assert!(
            task.is_visited(),
            "expect the visit to be transitioned to visited once visited is called"
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_simple_traverse() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependecy for 'A'");
        graph
            .add_dependency("B", "C")
            .expect("for task 'C' to be added as dependecy for 'B'");

        assert_eq!(
            graph
                .resolve("A")
                .expect("for the graph to be traverse without errors"),
            vec!["C", "B", "A"]
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_cylcle_dependecy() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependecy for 'A'");
        graph
            .add_dependency("B", "C")
            .expect("for task 'C' to be added as dependecy for 'B'");
        graph
            .add_dependency("C", "A")
            .expect("for task 'A' to be added as dependecy for 'C'");

        assert_eq!(
            graph.resolve("A").expect_err("to return a cycle dependecy"),
            GraphError::DependencyCylce(String::from("A"))
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_error_when_task_does_not_exist() {
        let graph = DependencyGraph::new();
        assert_eq!(
            graph
                .resolve("A")
                .expect_err("expect to error out when task does nto exist"),
            GraphError::UnknownTask(String::from("A"))
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_it_return_same_when_called_multiple_times() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependecy for 'A'");
        let first = graph
            .resolve("A")
            .expect("for resolve task not return any error");
        let second = graph
            .resolve("A")
            .expect("for resolve task not return any error");
        assert_eq!(first, vec!["B", "A"]);
        assert_eq!(second, vec!["B", "A"]);
    }

    #[test]
    fn test_dependecy_graph_resolve_it_returns_only_the_realted_tasks() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");
        graph.add_task("D").expect("to task 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependecy for 'A'");
        graph
            .add_dependency("B", "C")
            .expect("for task 'C' to be added as dependecy for 'B'");

        let result = graph.resolve("A").expect("for resolve A not to error out");
        assert_eq!(result, vec!["C", "B", "A"]);
        assert!(!result.iter().any(|id| id == "D"));
        assert_eq!(
            graph.resolve("D").expect("for resolve D not to error out"),
            vec!["D"]
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_it_returns_exactly_once_all_related_tasks() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");
        graph.add_task("D").expect("to task 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependecy for 'A'");
        graph
            .add_dependency("A", "C")
            .expect("for task 'C' to be added as dependecy for 'A'");
        graph
            .add_dependency("B", "D")
            .expect("for task 'D' to be added as dependecy for 'B'");
        graph
            .add_dependency("C", "D")
            .expect("for task 'D' to be added as dependecy for 'C'");
        assert_eq!(
            graph.resolve("A").expect("for resolve A not to error out"),
            vec!["D", "B", "C", "A"]
        );
    }
}
