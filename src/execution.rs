use std::{
    cmp,
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

enum QueueStatus {
    Open,
    Closed,
}

struct SharedQueue {
    queue: VecDeque<String>,
    status: QueueStatus,
}

impl SharedQueue {
    fn new(queue: VecDeque<String>) -> Self {
        Self {
            queue: queue,
            status: QueueStatus::Open,
        }
    }
}

#[derive(Debug)]
pub struct TaskResult {
    task_id: String,
    result: Result<(), TaskError>,
}
#[derive(Debug)]
pub enum TaskError {}
pub fn execute_layer<F>(tasks: Vec<String>, max_workers: usize, task_fn: F) -> Vec<TaskResult>
where
    F: Fn(&str) -> Result<(), TaskError> + Send + Sync + 'static,
{
    let mut queue: VecDeque<String> = VecDeque::new();
    for task in tasks.iter() {
        queue.push_back(task.clone());
    }

    let shared_queue: Arc<Mutex<SharedQueue>> = Arc::new(Mutex::new(SharedQueue::new(queue)));
    let workers = cmp::min(max_workers, tasks.len());
    let mut handlers: Vec<thread::JoinHandle<()>> = vec![];
    let task_fn: Arc<F> = Arc::new(task_fn);
    let (tx, rx): (Sender<TaskResult>, Receiver<TaskResult>) = mpsc::channel();
    for id in 0..workers {
        let shared_queue = Arc::clone(&shared_queue);
        let task_fn = Arc::clone(&task_fn);
        let worker_tx = tx.clone();
        handlers.push(thread::spawn(move || {
            loop {
                let next_task = shared_queue.lock().unwrap().queue.pop_front();
                match next_task {
                    Some(task) => {
                        println!("hello from another thread {id}-{:?}", task);
                        let result = task_fn(&task);
                        worker_tx
                            .send(TaskResult {
                                result: result,
                                task_id: task,
                            })
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

    let mut result: Vec<TaskResult> = Vec::with_capacity(tasks.len());
    while let Ok(res) = rx.recv() {
        result.push(res);
    }

    for handler in handlers {
        handler.join().unwrap();
    }

    println!("{:?}", result);
    result
}

#[cfg(test)]
mod execution_test {
    use super::*;

    #[test]
    fn test_execute_layer() {
        let exec = |task_id: &str| -> Result<(), TaskError> {
            println!("executing {task_id}");
            Ok(())
        };
        execute_layer(
            vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "X".to_string(),
            ],
            2,
            exec,
        );
    }
}
