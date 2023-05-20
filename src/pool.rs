use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use std::thread;
use std::sync::mpsc;
use std::sync;

struct Worker{
    id:usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker{
    fn new(id:usize, receiver:Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop{
            let job = receiver.lock().unwrap().recv().unwrap();
            println!("Worker {} got a job; executing.",id);
            job();
        });
        Worker {id,thread:Some(thread)}
    }

}

pub struct ThreadPool{
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size:usize) -> ThreadPool{
        assert!(size > 0);
        
        let (sender,receiver) = mpsc::channel();
        
        let mut workers = Vec::with_capacity(size);
        let receiver: Arc<Mutex<Receiver<Job>>> = Arc::new(Mutex::new(receiver));
        for id in 0..size{
            workers.push(Worker::new(id, receiver.clone()));
        }

        ThreadPool {workers, sender }
    }
    
    pub fn execute<F>(&self,f:F)
    where
        F: FnOnce() + Send + 'static
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }

}