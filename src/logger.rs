use crate::messages::constants::config::{LOG_FILE, QUIET, VERBOSE};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Logger {
    log_file: Arc<Mutex<std::fs::File>>,
    mode: String,
}

impl Logger {
    pub fn new(log_file: String, log_level: String) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .expect("Failed to open log file");
        Self {
            log_file: Arc::new(Mutex::new(file)),
            mode: log_level,
        }
    }

    pub fn default() -> Self {
        Self::new(LOG_FILE.to_owned(), QUIET.to_owned())
    }

    fn log_verbose(&self, message: &str) {
        if self.mode != VERBOSE {
            return;
        }
        let now = Local::now();
        let line = format!("{} - {}\n", now, message);
        eprintln!("{}", message);
        let mut file = self.log_file.lock().unwrap();
        file.write_all(line.as_bytes())
            .expect("Failed to write to log file")
    }

    fn log_quiet(&self, message: &str) {
        let now = Local::now();
        let line = format!("{} - {}\n", now, message);
        eprintln!("{}", message);
        let mut file = self.log_file.lock().unwrap();
        file.write_all(line.as_bytes())
            .expect("Failed to write to log file")
    }

    pub fn log(&self, message: &str, level: &str) {
        match level {
            VERBOSE => self.log_verbose(message),
            _ => self.log_quiet(message),
        }
    }
}
