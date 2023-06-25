use crate::messages::constants::config::VERBOSE;
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, mpsc, Mutex};
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
                    Log::Quiet(content) =>  writeln!(file, "{}", &content),
                    _ => Ok(()),
                };
            }
        });

        Logger {
            log_sender: sender,
        }
    }
}

//test logger
#[cfg(test)]
mod tests {
    use crate::logger::{Log, Logger};
    use crate::messages::constants::config::{QUIET, VERBOSE};
    use std::fs::File;
    use std::io::Read;
    use std::sync::mpsc::channel;
    use std::{fs, thread};

    #[test]
    fn test_logger() {
        let log_file = "test.log".to_string();
        let log_level = VERBOSE.to_string();
        let logger = Logger::new(log_file, log_level);
        logger.log_sender.send(Log::Verbose("test".to_string())).unwrap();
        logger.log_sender.send(Log::Verbose("test".to_string())).unwrap();

        let mut file = File::open("test.log").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        fs::remove_file("test.log").unwrap();
        assert_eq!(contents, "test\ntest\n");
    }

    #[test]
    fn test_logger_quiet() {
        let log_file = "test2.log".to_string();
        let log_level = QUIET.to_string();

        let logger = Logger::new(log_file, log_level);
        logger.log_sender.send(Log::Verbose("test".to_string())).unwrap();
        logger.log_sender.send(Log::Quiet("test".to_string())).unwrap();
        let mut file = File::open("test2.log").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        fs::remove_file("test.log").unwrap();
        assert_eq!(contents, "test\n");
    }
}




