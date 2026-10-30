use std::{collections::HashMap, result::Result};
use thiserror::Error;

pub struct Task {
    dependencies: Vec<String>,
}

#[derive(PartialEq, Eq, Debug, Error)]
pub enum GraphError {
    #[error("Task {0} already exist")]
    DuplicatedTask(String),
    #[error("Task {0} does not exist")]
    UnknownTask(String),
    #[error("Task {dependency_id} is already a dependency of {task_id}")]
    DuplicatedDependency {
        dependency_id: String,
        task_id: String,
    },
    #[error("A cycle was detected with {0}")]
    CycleDetected(String),
}

pub struct DependencyGraph {
    tasks: HashMap<String, Task>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, id: &str) -> Result<(), GraphError> {
        if self.tasks.contains_key(id) {
            return Err(GraphError::DuplicatedTask(id.to_string()));
        }

        let key = id.to_string();
        self.tasks.insert(
            key,
            Task {
                dependencies: Vec::new(),
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
        let already_dependency = task.dependencies.iter().any(|id| id == dependency_id);
        if already_dependency {
            return Err(GraphError::DuplicatedDependency {
                dependency_id: dependency_id.to_string(),
                task_id: task_id.to_string(),
            });
        }

        task.dependencies.push(dependency_id.to_owned());
        Ok(())
    }

    pub fn resolve_many(&self, roots: &[&str]) -> Result<Vec<String>, GraphError> {
        let mut result: Vec<String> = Vec::new();
        let mut stack: Vec<(&str, StackOperation)> = Vec::new();
        let mut pipeline: HashMap<&str, TraverseState> = HashMap::new();
        for root in roots {
            self.tasks
                .contains_key(*root)
                .ok_or(GraphError::UnknownTask(root.to_string()))?;
            stack.push((root, StackOperation::Entering));
        }
        while let Some((key, operation)) = stack.pop() {
            match operation {
                StackOperation::Exiting => {
                    pipeline.insert(key, TraverseState::Visited);
                    result.push(key.to_string());
                }
                StackOperation::Entering => {}
            }

            match pipeline.get(key) {
                Some(TraverseState::Visited) => {
                    continue;
                }
                Some(TraverseState::Visiting) => {
                    return Err(GraphError::CycleDetected(key.to_string()));
                }
                None => {}
            }
            let task = self
                .tasks
                .get(key)
                .ok_or(GraphError::UnknownTask(key.to_string()))?;
            pipeline.insert(key, TraverseState::Visiting);
            stack.push((key, StackOperation::Exiting));
            for dependency in task.dependencies.iter() {
                stack.push((dependency, StackOperation::Entering));
            }
        }

        Ok(result)
    }

    pub fn resolve(&self, task_id: &str) -> Result<Vec<String>, GraphError> {
        let mut result: Vec<String> = Vec::new();
        let mut pipeline: HashMap<String, TraverseState> = HashMap::new();

        self.visit_task(task_id, &mut result, &mut pipeline)?;

        Ok(result)
    }

    pub fn resolve_iterative(&self, task_id: &str) -> Result<Vec<String>, GraphError> {
        let mut result: Vec<String> = Vec::new();
        // this 2 are deliverated using &str instead of String, this is to avoid
        // unecessary allocations to imporve performance while travesig the graph.
        let mut stack: Vec<(&str, StackOperation)> =
            Vec::from([(task_id, StackOperation::Entering)]);
        let mut pipeline: HashMap<&str, TraverseState> = HashMap::new();

        while let Some((id, operation)) = stack.pop() {
            match operation {
                StackOperation::Exiting => {
                    pipeline.insert(id, TraverseState::Visited);
                    result.push(id.to_string());
                    continue;
                }
                StackOperation::Entering => {}
            }

            match pipeline.get(id) {
                Some(TraverseState::Visiting) => {
                    return Err(GraphError::CycleDetected(id.to_string()));
                }
                Some(TraverseState::Visited) => {
                    continue;
                }
                None => {}
            }

            let task = self
                .tasks
                .get(id)
                .ok_or(GraphError::UnknownTask(id.to_string()))?;
            pipeline.insert(id, TraverseState::Visiting);
            stack.push((id, StackOperation::Exiting));

            for dependency in task.dependencies.iter().rev() {
                stack.push((dependency, StackOperation::Entering));
            }
        }

        Ok(result)
    }

    fn visit_task(
        &self,
        task_id: &str,
        result: &mut Vec<String>,
        pipeline: &mut HashMap<String, TraverseState>,
    ) -> Result<(), GraphError> {
        // 1.- task exist
        let task = self
            .tasks
            .get(task_id)
            .ok_or(GraphError::UnknownTask(task_id.to_string()))?;

        match pipeline.get(task_id) {
            // 2.- is visiting?
            Some(TraverseState::Visiting) => {
                return Err(GraphError::CycleDetected(task_id.to_string()));
            }
            // 3.- is visited?
            Some(TraverseState::Visited) => return Ok(()),
            None => {}
        };
        // 4.- add into the pipeline
        pipeline.insert(task_id.to_string(), TraverseState::Visiting);

        // 5.- traverse dependencies.
        let dependencies = &task.dependencies;
        for dependency in dependencies {
            self.visit_task(dependency, result, pipeline)?;
        }

        // 6.- visited
        pipeline.insert(task_id.to_string(), TraverseState::Visited);
        result.push(task_id.to_string());

        Ok(())
    }
}

#[derive(PartialEq, Debug)]
enum TraverseState {
    Visiting,
    Visited,
}

#[derive(Debug, PartialEq)]
enum StackOperation {
    Entering,
    Exiting,
}

#[cfg(test)]
mod graph_test {
    use std::vec;

    use super::*;

    #[test]
    fn test_dependency_graph_add_task() {
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
    fn test_dependency_graph_add_task_duplicated_task() {
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
            "expect only a single element in the tasks for the graph"
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
                    task.dependencies,
                    vec!["B"],
                    "to 'B' to be present in the dependencies array"
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
        let missing_dependency_err = graph.add_dependency("A", "B");
        assert!(
            missing_dependency_err.is_err(),
            "expect to fail because missing dependency task"
        );
        assert_eq!(
            missing_dependency_err.err(),
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
            .expect("for 'B' to be added as dependency on 'A'");
        assert_eq!(
            graph.tasks.get("A").expect("'A' to exist").dependencies,
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
            graph.tasks.get("A").expect("'A' to exist").dependencies,
            vec!["B"]
        );
    }

    #[test]
    fn test_dependency_graph_resolve_simple_traverse() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependency for 'A'");
        graph
            .add_dependency("B", "C")
            .expect("for task 'C' to be added as dependency for 'B'");

        assert_eq!(
            graph
                .resolve("A")
                .expect("for the graph to be traverse without errors"),
            vec!["C", "B", "A"]
        );
    }

    #[test]
    fn test_dependency_graph_resolve_cycle_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependency for 'A'");
        graph
            .add_dependency("B", "C")
            .expect("for task 'C' to be added as dependency for 'B'");
        graph
            .add_dependency("C", "A")
            .expect("for task 'A' to be added as dependency for 'C'");

        assert_eq!(
            graph
                .resolve("A")
                .expect_err("to return a cycle dependency"),
            GraphError::CycleDetected(String::from("A"))
        );
    }

    #[test]
    fn test_dependency_graph_resolve_error_when_task_does_not_exist() {
        let graph = DependencyGraph::new();
        assert_eq!(
            graph
                .resolve("A")
                .expect_err("expect to error out when task does not exist"),
            GraphError::UnknownTask(String::from("A"))
        );
    }

    #[test]
    fn test_dependency_graph_resolve_it_return_same_when_called_multiple_times() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependency for 'A'");
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
    fn test_dependency_graph_resolve_it_returns_only_the_related_tasks() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");
        graph.add_task("D").expect("to task 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependency for 'A'");
        graph
            .add_dependency("B", "C")
            .expect("for task 'C' to be added as dependency for 'B'");

        let result = graph.resolve("A").expect("for resolve A not to error out");
        assert_eq!(result, vec!["C", "B", "A"]);
        assert!(!result.iter().any(|id| id == "D"));
        assert_eq!(
            graph.resolve("D").expect("for resolve D not to error out"),
            vec!["D"]
        );
    }

    #[test]
    fn test_dependency_graph_resolve_it_returns_exactly_once_all_related_tasks() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");
        graph.add_task("D").expect("to task 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependency for 'A'");
        graph
            .add_dependency("A", "C")
            .expect("for task 'C' to be added as dependency for 'A'");
        graph
            .add_dependency("B", "D")
            .expect("for task 'D' to be added as dependency for 'B'");
        graph
            .add_dependency("C", "D")
            .expect("for task 'D' to be added as dependency for 'C'");
        assert_eq!(
            graph.resolve("A").expect("for resolve A not to error out"),
            vec!["D", "B", "C", "A"]
        );
    }

    #[test]
    fn test_dependency_graph_resolve_self_dependencies_and_unrelated_solves() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("X").expect("to task 'X' to be added");
        graph.add_task("Y").expect("to task 'Y' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for dependency 'B' to be added to A");
        graph
            .add_dependency("X", "Y")
            .expect("for dependency 'Y' to be added to 'X'");
        graph
            .add_dependency("X", "X")
            .expect("for dependency 'X' to be added to 'X'");

        assert_eq!(
            graph.resolve("A").expect("for resolve A to not error out"),
            vec!["B", "A"]
        );
        assert_eq!(
            graph
                .resolve("X")
                .expect_err("for resolve X to fail with cycle detected"),
            GraphError::CycleDetected(String::from("X"))
        );
    }

    #[test]
    fn test_dependency_graph_resolve_iterative_same_as_resolve() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");
        graph.add_task("D").expect("to task 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependency for 'A'");
        graph
            .add_dependency("A", "C")
            .expect("for task 'C' to be added as dependency for 'A'");
        graph
            .add_dependency("B", "D")
            .expect("for task 'D' to be added as dependency for 'B'");
        graph
            .add_dependency("C", "D")
            .expect("for task 'D' to be added as dependency for 'C'");

        let iterative_res = graph
            .resolve_iterative("A")
            .expect("for iterative not to error out");
        let recursive_res = graph
            .resolve("A")
            .expect("for resolve not to error error out");

        assert_eq!(iterative_res, recursive_res);
        assert_eq!(iterative_res, vec!["D", "B", "C", "A"]);
        assert_eq!(recursive_res, vec!["D", "B", "C", "A"]);
    }

    #[test]
    fn test_dependency_graph_resolve_iterative_handles_cycle_detections() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");

        graph
            .add_dependency("A", "A")
            .expect("to 'A' is dependency for 'A'");

        graph
            .add_dependency("B", "C")
            .expect("fo a 'C' becomes a dependency for 'B'");
        graph
            .add_dependency("C", "B")
            .expect("for 'B' to be a dependency of 'C'");

        assert_eq!(
            graph
                .resolve_iterative("A")
                .expect_err("to faile because cycle"),
            GraphError::CycleDetected(String::from("A"))
        );

        assert_eq!(
            graph
                .resolve_iterative("B")
                .expect_err("To fail for a cycle in B"),
            GraphError::CycleDetected(String::from("B"))
        );
    }

    #[test]
    fn test_dependency_graph_resolve_iterative_only_returns_related_tasks() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("X").expect("to task 'X' to be added");
        graph.add_task("Y").expect("to task 'Y' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for dependency 'B' to be added to A");
        graph
            .add_dependency("X", "Y")
            .expect("for dependency 'Y' to be added to 'X'");

        assert_eq!(
            graph
                .resolve_iterative("A")
                .expect("for resolve_iterative(A) not to error out "),
            vec!["B", "A"]
        );

        assert_eq!(
            graph
                .resolve_iterative("X")
                .expect("for resolve_iterative(X) not to error out "),
            vec!["Y", "X"]
        );
    }

    #[test]
    fn test_dependency_graph_resolve_iterative_return_erro_when_task_missing() {
        let graph = DependencyGraph::new();
        assert_eq!(
            graph
                .resolve_iterative("A")
                .expect_err("with missing task 'A'"),
            GraphError::UnknownTask(String::from("A"))
        );
    }

    #[test]
    fn test_dependency_graph_resolve_iterative_iterates_over_related_paths() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("X").expect("to task 'X' to be added");
        graph.add_task("Y").expect("to task 'Y' to be added");

        graph
            .add_dependency("A", "A")
            .expect("to create a self dependency");
        graph
            .add_dependency("X", "Y")
            .expect("for 'X' to be dependent of 'Y'");

        assert_eq!(
            graph
                .resolve_iterative("A")
                .expect_err("for erro out self dependency"),
            GraphError::CycleDetected(String::from("A"))
        );

        assert_eq!(
            graph
                .resolve_iterative("X")
                .expect("for 'X' not to be affected by the cycle on 'A'"),
            vec!["Y", "X"]
        );
    }

    #[test]
    fn test_dependency_graph_dependency_order_matters() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("to task 'A' to be added");
        graph.add_task("B").expect("to task 'B' to be added");
        graph.add_task("C").expect("to task 'C' to be added");
        graph.add_task("D").expect("to task 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for task 'B' to be added as dependency for 'A'");
        graph
            .add_dependency("A", "C")
            .expect("for task 'C' to be added as dependency for 'A'");
        graph
            .add_dependency("B", "D")
            .expect("for task 'D' to be added as dependency for 'B'");
        graph
            .add_dependency("C", "D")
            .expect("for task 'D' to be added as dependency for 'C'");

        assert_eq!(
            graph.resolve("A").expect("for recursive solution to work"),
            graph
                .resolve_iterative("A")
                .expect("for iterative solutioon to work ")
        );
        assert_eq!(
            graph.resolve("A").expect("for recursive solution to work"),
            vec!["D", "B", "C", "A"]
        );
        assert_eq!(
            graph
                .resolve_iterative("A")
                .expect("for recursive solution to work"),
            vec!["D", "B", "C", "A"]
        );

        let mut graph2 = DependencyGraph::new();
        graph2
            .add_task("A")
            .expect("graph2: to task 'A' to be added");
        graph2
            .add_task("B")
            .expect("graph2: to task 'B' to be added");
        graph2
            .add_task("C")
            .expect("graph2: to task 'C' to be added");
        graph2
            .add_task("D")
            .expect("graph2: to task 'D' to be added");

        graph2
            .add_dependency("A", "C")
            .expect("graph2: for task 'C' to be added as dependency for 'A'");
        graph2
            .add_dependency("A", "B")
            .expect("graph2: for task 'B' to be added as dependency for 'A'");
        graph2
            .add_dependency("B", "D")
            .expect("graph2: for task 'D' to be added as dependency for 'B'");
        graph2
            .add_dependency("C", "D")
            .expect("graph2: for task 'D' to be added as dependency for 'C'");

        assert_eq!(
            graph2
                .resolve("A")
                .expect("graph2: for recursive solution to work"),
            graph2
                .resolve_iterative("A")
                .expect("graph2: for iterative solutioon to work ")
        );
        assert_eq!(
            graph2
                .resolve("A")
                .expect("graph2: for recursive solution to work"),
            vec!["D", "C", "B", "A"]
        );
        assert_eq!(
            graph2
                .resolve_iterative("A")
                .expect("graph2: for recursive solution to work"),
            vec!["D", "C", "B", "A"]
        );

        assert_ne!(
            graph
                .resolve_iterative("A")
                .expect("graph: to be able to solve dependency of A"),
            graph2
                .resolve_iterative("A")
                .expect("graph2: to be able to solve dependency of A")
        );
    }

    #[test]
    fn test_dependency_graph_resolve_many_only_resolve_same_root_one_time() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("for 'A' to be added");
        graph.add_task("B").expect("for 'B' to be added");
        graph.add_task("C").expect("for 'C' to be added");
        graph
            .add_dependency("A", "B")
            .expect("for a new depency to be created");
        graph
            .add_dependency("B", "C")
            .expect("for a new depency to be created");

        assert_eq!(
            graph
                .resolve_many(&["A", "A"])
                .expect("for resolve many to return the result of only 'A'"),
            vec!["C", "B", "A"]
        );
    }

    #[test]
    fn tes_dependency_graph_resolve_many_resolve_independent_sequences() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("for 'A' to be added");
        graph.add_task("B").expect("for 'B' to be added");
        graph.add_task("C").expect("for 'C' to be added");
        graph
            .add_dependency("A", "B")
            .expect("for a new depency to be created");
        graph
            .add_dependency("B", "C")
            .expect("for a new depency to be created");
        graph.add_task("X").expect("for 'X' to be added");
        graph.add_task("Y").expect("for 'Y' to be added");
        graph
            .add_dependency("X", "Y")
            .expect("for a new dependency to be created");

        assert_eq!(
            graph
                .resolve_many(&["A", "X"])
                .expect("for resolve_many to return the result"),
            vec!["Y", "X", "C", "B", "A"]
        );

        assert_eq!(
            graph
                .resolve_many(&["X", "A"])
                .expect("for resolve_many to return the result"),
            vec!["C", "B", "A", "Y", "X"]
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_many_resole_related_sequences() {
        let mut graph = DependencyGraph::new();
        graph.add_task("A").expect("for 'A' to be added");
        graph.add_task("B").expect("for 'B' to be added");
        graph.add_task("X").expect("for 'X' to be added");
        graph.add_task("Y").expect("for 'Y' to be added");
        graph.add_task("C").expect("for 'C' to be added");
        graph.add_task("D").expect("for 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("B", "C")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("C", "D")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("X", "Y")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("Y", "C")
            .expect("for a new dependecy to be created");

        assert_eq!(
            graph
                .resolve_many(&["A", "X"])
                .expect("for resolve_manyb to return a result"),
            vec!["D", "C", "Y", "X", "B", "A"]
        );
        assert_eq!(
            graph
                .resolve_many(&["X", "A"])
                .expect("for resolve_manyb to return a result"),
            vec!["D", "C", "B", "A", "Y", "X"]
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_many_return_empty_when_nothing_requested() {
        let mut graph: DependencyGraph = DependencyGraph::new();
        graph.add_task("A").expect("for 'A' to be added");
        graph.add_task("B").expect("for 'B' to be added");
        graph.add_task("X").expect("for 'X' to be added");
        graph.add_task("Y").expect("for 'Y' to be added");
        graph.add_task("C").expect("for 'C' to be added");
        graph.add_task("D").expect("for 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("B", "C")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("C", "D")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("X", "Y")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("Y", "C")
            .expect("for a new dependecy to be created");

        let expected: Vec<String> = Vec::new();
        assert_eq!(
            graph
                .resolve_many(&[])
                .expect("for empty slice to be valid"),
            expected
        );
    }

    #[test]
    fn test_dependecy_graph_can_detect_unknow_tasks() {
        let mut graph: DependencyGraph = DependencyGraph::new();
        graph.add_task("A").expect("for 'A' to be added");
        graph.add_task("B").expect("for 'B' to be added");
        graph.add_task("X").expect("for 'X' to be added");
        graph.add_task("Y").expect("for 'Y' to be added");
        graph.add_task("C").expect("for 'C' to be added");
        graph.add_task("D").expect("for 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("B", "C")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("C", "D")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("X", "Y")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("Y", "C")
            .expect("for a new dependecy to be created");

        assert_eq!(
            graph
                .resolve_many(&["A", "Z"])
                .expect_err("to error out when unknow task is requested"),
            GraphError::UnknownTask(String::from("Z"))
        );
    }

    #[test]
    fn test_dependecy_graph_resolve_many_can_detect_cycles() {
        let mut graph: DependencyGraph = DependencyGraph::new();
        graph.add_task("A").expect("for 'A' to be added");
        graph.add_task("B").expect("for 'B' to be added");
        graph.add_task("X").expect("for 'X' to be added");
        graph.add_task("Y").expect("for 'Y' to be added");
        graph.add_task("C").expect("for 'C' to be added");
        graph.add_task("D").expect("for 'D' to be added");

        graph
            .add_dependency("A", "B")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("B", "C")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("C", "D")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("X", "Y")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("Y", "C")
            .expect("for a new dependecy to be created");
        graph
            .add_dependency("D", "A")
            .expect("for a new dependecy to be created");

        assert_eq!(
            graph
                .resolve_many(&["A", "X"])
                .expect_err("to found a dependecy cycle"),
            GraphError::CycleDetected(String::from("C"))
        );
    }
}

#[cfg(test)]
mod scale_test {
    use super::*;

    fn linear_chain(len: usize) -> (DependencyGraph, String) {
        assert!(len > 0);
        let mut graph = DependencyGraph::new();
        for i in 0..len {
            graph
                .add_task(&format!("n{i}"))
                .expect("for a new node to be added");
        }
        for i in 0..len.saturating_sub(1) {
            graph
                .add_dependency(&format!("n{i}"), &format!("n{}", i + 1))
                .expect("for a new dependency to be formed");
        }
        (graph, "n0".to_string())
    }

    fn layered_dag(layers: usize, width: usize) -> (DependencyGraph, String) {
        assert!(layers > 0);
        assert!(width > 0);
        let mut graph = DependencyGraph::new();
        graph
            .add_task("root")
            .expect("for the root node to be added");
        for l in 0..layers {
            for w in 0..width {
                graph
                    .add_task(&format!("l{l}_w{w}"))
                    .expect("for a new node to be added");
            }
        }

        for w in 0..width {
            graph
                .add_dependency("root", &format!("l0_w{w}"))
                .expect("for a dependency in root to be added");
        }

        for l in 0..layers.saturating_sub(1) {
            for w in 0..width {
                for nw in 0..width {
                    graph
                        .add_dependency(&format!("l{l}_w{w}"), &format!("l{}_w{nw}", l + 1))
                        .expect("for a new dependency layered to be added");
                }
            }
        }

        (graph, "root".to_string())
    }

    #[test]
    fn iterative_and_recursive_solve_linear_100() {
        let (graph, root) = linear_chain(100);
        let iterative = graph
            .resolve_iterative(&root)
            .expect("for iteative solution");
        let recursive = graph.resolve(&root).expect("for recursive solution");

        assert_eq!(
            iterative, recursive,
            "expect same result for iterative or recursive"
        );

        assert_valid_resolution(&graph, &iterative);
        assert_valid_resolution(&graph, &recursive);
    }

    #[test]
    fn iterative_and_recursive_solve_linear_1000() {
        let (graph, root) = linear_chain(1000);
        let iterative = graph
            .resolve_iterative(&root)
            .expect("for iteative solution");
        let recursive = graph.resolve(&root).expect("for recursive solution");

        assert_eq!(
            iterative, recursive,
            "expect same result for iterative or recursive"
        );

        assert_valid_resolution(&graph, &iterative);
        assert_valid_resolution(&graph, &recursive);
    }

    #[test]
    #[ignore = "Large graph stress test"]
    fn iterative_handles_deep_chains() {
        let (graph, root) = linear_chain(200_000);
        let iterative = graph
            .resolve_iterative(&root)
            .expect("for iteative solution");
        assert_valid_resolution(&graph, &iterative);
    }

    #[test]
    fn iterative_and_recursive_solve_dag_5_10() {
        let (graph, root) = layered_dag(5, 10);
        let iterative = graph
            .resolve_iterative(&root)
            .expect("for a iterative solution in dag");
        let recursive = graph.resolve(&root).expect("for recusrive solution in dag");

        assert_eq!(recursive, iterative);
        assert_valid_resolution(&graph, &iterative);
        assert_valid_resolution(&graph, &recursive);
    }

    #[test]
    fn iterative_recursive_solve_dag_10_50() {
        let (graph, root) = layered_dag(10, 50);
        let iterative = graph
            .resolve_iterative(&root)
            .expect("for a iterative solution in dag");
        let recursive = graph.resolve(&root).expect("for recusrive solution in dag");

        assert_eq!(recursive, iterative);
        assert_valid_resolution(&graph, &iterative);
        assert_valid_resolution(&graph, &recursive);
    }

    fn assert_valid_resolution(graph: &DependencyGraph, result: &[String]) {
        assert_eq!(
            graph.tasks.len(),
            result.len(),
            "expect for the result len to be equal to the len of all tasks in the graph"
        );

        let mut positions: HashMap<&String, usize> = HashMap::new();
        for (index, key) in result.iter().enumerate() {
            assert!(
                positions.insert(key, index).is_none(),
                "Task {key} appears more than once"
            );
        }

        for (key, task) in graph.tasks.iter() {
            let index = positions
                .get(key)
                .unwrap_or_else(|| panic!("for {key} to be present in the result"));
            for dependency in task.dependencies.iter() {
                let dependency_index = positions.get(dependency).unwrap_or_else(||panic!("for all dependencies be present before the task is reached, task:{key}, missing dependency: {dependency}"));

                assert!(
                    dependency_index < &index,
                    "the dependency must be before the task, task: {key}, position: {index}, dependency: {dependency}, dependency_position: {dependency_index}"
                );
            }
        }
    }
}
