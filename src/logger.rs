use crate::messages::constants::config::VERBOSE;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

#[derive(Clone)]
pub struct Logger {
    pub log_sender: Sender<Log>,
}

pub enum Log {
    Verbose(String),
    Quiet(String),
}

impl Logger {
    pub fn new(log_file: String, log_level: String) -> Self {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .expect("Failed to open log file");

        let (sender, receiver): (Sender<Log>, Receiver<Log>) = mpsc::channel();
        thread::spawn(move || {
            let verbose = log_level == VERBOSE;
            for content in receiver.iter() {
                let _ = match content {
                    Log::Verbose(content) if verbose => writeln!(file, "{}", &content),
                    Log::Quiet(content) => writeln!(file, "{}", &content),
                    _ => Ok(()),
                };
            }
        });

        Logger { log_sender: sender }
    }
}
