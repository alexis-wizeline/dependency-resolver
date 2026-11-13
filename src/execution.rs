use std::{
    cmp,
    collections::VecDeque,
    panic::{self, RefUnwindSafe},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

struct SharedQueue {
    queue: VecDeque<String>,
}

impl SharedQueue {
    fn new(queue: VecDeque<String>) -> Self {
        Self { queue }
    }
}

#[derive(Debug)]
pub enum TaskError {
    FailedExecution(String),
}

#[derive(Debug)]
pub enum TaskOutcome {
    Succes,
    Falied(TaskError),
    Panicked,
    Nonstarted,
}

#[derive(Debug)]
pub struct TaskExecutionResult {
    pub task_id: String,
    pub outcome: TaskOutcome,
}

pub struct ExecutionReport {
    pub tasks: Vec<TaskExecutionResult>,
}

pub fn execute_layer<F>(tasks: Vec<String>, max_workers: usize, task_fn: F) -> ExecutionReport
where
    F: Fn(&str) -> Result<(), TaskError> + Send + Sync + RefUnwindSafe + 'static,
{
    assert!(max_workers > 0, "max_workers must be more than 0");

    let mut queue: VecDeque<String> = VecDeque::new();
    for task in tasks.iter() {
        queue.push_back(task.clone());
    }

    let shared_queue: Arc<Mutex<SharedQueue>> = Arc::new(Mutex::new(SharedQueue::new(queue)));
    let workers = cmp::min(max_workers, tasks.len());
    let mut handlers: Vec<thread::JoinHandle<()>> = vec![];
    let task_fn: Arc<F> = Arc::new(task_fn);
    let (tx, rx): (Sender<TaskExecutionResult>, Receiver<TaskExecutionResult>) = mpsc::channel();
    for id in 0..workers {
        let shared_queue = Arc::clone(&shared_queue);
        let task_fn = Arc::clone(&task_fn);
        let worker_tx = tx.clone();
        handlers.push(thread::spawn(move || {
            loop {
                let next_task = shared_queue.lock().unwrap().queue.pop_front();
                match next_task {
                    Some(task_id) => {
                        println!("hello from another thread {id}-{:?}", task_id);
                        let outcome: TaskOutcome;
                        match panic::catch_unwind(|| task_fn(&task_id)) {
                            Ok(res) => match res {
                                Ok(_) => outcome = TaskOutcome::Succes,
                                Err(err) => outcome = TaskOutcome::Falied(err),
                            },
                            Err(_) => outcome = TaskOutcome::Panicked,
                        }
                        worker_tx
                            .send(TaskExecutionResult { outcome, task_id })
                            .unwrap();
                    }
                    None => {
                        println!("{id} exiting");
                        break;
                    }
                }
            }
        }));
    }

    drop(tx);

    let mut task_report: Vec<TaskExecutionResult> = Vec::with_capacity(tasks.len());
    while let Ok(res) = rx.recv() {
        task_report.push(res);
    }

    for handler in handlers {
        match handler.join() {
            Ok(_) => {
                println!("thread execution ended without problems");
            }
            Err(err) => {
                if let Some(msg) = err.downcast_ref::<&'static str>() {
                    println!("thread was unable to be completed: {}", msg);
                } else if let Some(msg) = err.downcast_ref::<String>() {
                    println!("thread was unable to be completed: {}", msg);
                } else {
                    println!("thread was unable to be completed");
                }
            }
        }
    }

    println!("{:?}", task_report);
    ExecutionReport { tasks: task_report }
}

#[cfg(test)]
mod execution_test {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_execute_layer_all_tasks_are_prsent_only_once() {
        let exec = |task_id: &str| -> Result<(), TaskError> {
            println!("executing {task_id}");
            if task_id == "task_1" {
                return Err(TaskError::FailedExecution(
                    "i just don't want to".to_string(),
                ));
            } else if task_id == "task_8" {
                panic!("uh i died");
            }
            Ok(())
        };
        let task_number = 10;
        let mut tasks = vec![];
        let mut key_set: HashSet<String> = HashSet::new();
        for number in 0..task_number {
            tasks.push(format!("task_{}", number));
            key_set.insert(format!("task_{}", number));
        }
        let report = execute_layer(tasks, 3, exec);
        assert_eq!(
            task_number,
            report.tasks.len(),
            "the result must include a result for all tasks"
        );
        let mut result_set: HashSet<&str> = HashSet::new();
        for result in report.tasks.iter() {
            let task_id = result.task_id.as_str();
            assert!(
                result_set.insert(task_id),
                "{task_id} must be executed only once"
            );
            assert!(
                key_set.contains(task_id),
                "{task_id} must be part of the input keys"
            );
        }
    }
}
