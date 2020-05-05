use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

struct Worker {
  id: usize,
  thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
  fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Signal>>>) -> Worker {
    let thread = thread::spawn(move || {
      loop {
        let message = receiver.lock().unwrap().recv().unwrap();

        match message {
          Signal::JobWrapper(job) => {
            println!("Worker {} got a job; executing.", id);

            job();
          },
          Signal::Terminate => {
            println!("Recieved signal to close worker {}", id);

            break;
          }
        }
      }
    });

    Worker {
      id,
      thread: Some(thread),
    }
  }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Signal {
  JobWrapper(Job),
  Terminate,
}

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: mpsc::Sender<Signal>,
}

impl ThreadPool { 
  /// Create a new ThreadPool.
  ///
  /// The size is the number of threads in the pool.
  ///
  /// # Panics
  ///
  /// The `new` function will panic if the size is zero.
  /// TODO: return Result<ThreadPool, PoolCreationError>
  pub fn new(size: usize) -> ThreadPool {
    assert!(size > 0);

    let (sender, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));
    let mut workers = Vec::with_capacity(size);

    for id in 0..size {
      workers.push(Worker::new(id, Arc::clone(&receiver)));
    }
  
    ThreadPool {
      workers,
      sender
    }
  }

  pub fn execute<F>(&self, f: F)
  where
    F: FnOnce() + Send + 'static,
  { 
    let job = Box::new(f);

    self.sender.send(Signal::JobWrapper(job)).unwrap();
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    println!("Sending terminate signal to all workers.");

    for _ in &self.workers {
      self.sender.send(Signal::Terminate).unwrap();
    }

    println!("Shutting down all workers.");

    for worker in &mut self.workers {
      println!("Shutting down worker {}", worker.id);

      if let Some(thread) = worker.thread.take() {
        thread.join().unwrap();
      }
    }
  }
}