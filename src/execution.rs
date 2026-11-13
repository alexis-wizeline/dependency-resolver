use std::{
    cmp,
    collections::{HashMap, VecDeque},
    panic::{RefUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
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

#[derive(Debug, PartialEq, Eq)]
pub enum TaskError {
    FailedExecution(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum TaskOutcome {
    Succes,
    Falied(TaskError),
    Panicked,
    NonStarted,
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
    let mut task_report: HashMap<&str, TaskExecutionResult> = HashMap::with_capacity(tasks.len());
    for task in tasks.iter() {
        queue.push_back(task.clone());
        task_report.insert(
            task,
            TaskExecutionResult {
                task_id: task.clone(),
                outcome: TaskOutcome::NonStarted,
            },
        );
    }

    let shared_queue: Arc<Mutex<SharedQueue>> = Arc::new(Mutex::new(SharedQueue::new(queue)));
    let workers = cmp::min(max_workers, tasks.len());
    let mut handlers: Vec<thread::JoinHandle<()>> = vec![];
    let task_fn: Arc<F> = Arc::new(task_fn);
    let (tx, rx): (Sender<TaskExecutionResult>, Receiver<TaskExecutionResult>) = channel();
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
                        match catch_unwind(|| task_fn(&task_id)) {
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

    while let Ok(res) = rx.recv() {
        if let Some(report) = task_report.get_mut(res.task_id.as_str()) {
            report.outcome = res.outcome
        }
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

    ExecutionReport {
        tasks: task_report.into_values().collect(),
    }
}

#[cfg(test)]
mod execution_test {
    use std::{
        collections::HashSet,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use super::*;

    #[test]
    fn test_execute_layer_all_tasks_are_present_only_once() {
        let exec = |task_id: &str| -> Result<(), TaskError> {
            println!("executing {task_id}");
            Ok(())
        };
        let task_number = 45;
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

    #[test]
    fn test_execute_layer_respects_concurrency_limit() {
        for max_workers in [1, 2, 4] {
            let task_count = 32;
            let tasks: Vec<String> = (0..task_count).map(|i| format!("task_{i}")).collect();

            let active = Arc::new(AtomicUsize::new(0));
            let peak = Arc::new(AtomicUsize::new(0));

            let task_active = Arc::clone(&active);
            let task_peak = Arc::clone(&peak);

            let report = execute_layer(tasks, max_workers, move |_| {
                let running = task_active.fetch_add(1, Ordering::SeqCst) + 1;
                task_peak.fetch_max(running, Ordering::SeqCst);

                thread::sleep(Duration::from_millis(10));
                task_active.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            });

            let observed_peak = peak.load(Ordering::SeqCst);

            assert!(
                observed_peak <= max_workers,
                "limit was {max_workers}, but {observed_peak} tasks ran simultaneosly"
            );
            assert!(observed_peak > 0);
            assert_eq!(active.load(Ordering::SeqCst), 0);
            assert_eq!(report.tasks.len(), task_count);
            assert!(
                report
                    .tasks
                    .iter()
                    .all(|res| { matches!(&res.outcome, TaskOutcome::Succes) }),
                "all tasks should succeed"
            );
        }
    }

    #[test]
    fn test_execute_layer_collects_all_results() {
        let task_count = 32;
        let mut outcome_map: HashMap<String, TaskOutcome> = HashMap::new();
        let mut tasks: Vec<String> = Vec::new();
        for i in 0..task_count {
            tasks.push(format!("Task_{i}"));
            match i % 3 {
                0 => {
                    outcome_map.insert(format!("Task_{i}"), TaskOutcome::Panicked);
                }
                2 => {
                    outcome_map.insert(
                        format!("Task_{i}"),
                        TaskOutcome::Falied(TaskError::FailedExecution(String::from(""))),
                    );
                }
                _ => {
                    outcome_map.insert(format!("Task_{i}"), TaskOutcome::Succes);
                }
            }
        }

        let outcome_map = Arc::new(outcome_map);
        let task_outcome_map = Arc::clone(&outcome_map);
        let report = execute_layer(tasks, 3, move |task_id| {
            match task_outcome_map.get(task_id) {
                Some(TaskOutcome::Falied(_)) => {
                    return Err(TaskError::FailedExecution(String::from("")));
                }
                Some(TaskOutcome::Panicked) => panic!("you say die I died"),
                _ => return Ok(()),
            }
        });

        assert_eq!(report.tasks.len(), task_count);
        for t in report.tasks.iter() {
            assert_eq!(
                &t.outcome,
                outcome_map
                    .get(&t.task_id)
                    .unwrap_or(&TaskOutcome::NonStarted)
            );
        }
    }
}
