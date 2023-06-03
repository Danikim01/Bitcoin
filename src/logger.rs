use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex, Once};
use chrono::Local;
use crate::config::Config;
use crate::messages::constants::config::VERBOSE;

struct Logger {
    log_file: Arc<Mutex<std::fs::File>>,
    mode: String,
}

impl Logger {
    fn new() -> Logger {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("src/log.txt").expect("Failed to open log file");
        Logger {
            log_file: Arc::new(Mutex::new(file)),
            mode: Config::from_file_or_default().get_logger_mode(),
        }
    }

    fn log_verbose(&self, message: &str) {
        let now = Local::now();
        let line = format!("{} - {}\n", now, message);
        println!("{}", message);

        if self.mode != VERBOSE { return; }
        let mut file = self.log_file.lock().unwrap();
        file.write_all(line.as_bytes()).expect("Failed to write to log file")
    }

    fn log_quiet(&self, message: &str) {
        let now = Local::now();
        let line = format!("{} - {}\n", now, message);
        println!("{}", message);
        let mut file = self.log_file.lock().unwrap();
        file.write_all(line.as_bytes()).expect("Failed to write to log file")
    }

}



struct LazyLogger {
    once: Once,
    logger: Option<Logger>,
}

impl LazyLogger {
    fn new() -> LazyLogger {
        LazyLogger {
            once: Once::new(),
            logger: None,
        }
    }

    fn get_logger(&mut self) -> &Logger {
        self.once.call_once(|| {
            let logger = Logger::new();
            self.logger = Some(logger);
        });
        self.logger.as_ref().unwrap()
    }
}

pub fn log(message: &str, mode: &str) {
    let mut lazy_logger = LazyLogger::new();
    let logger = lazy_logger.get_logger();

    match mode{
        VERBOSE => logger.log_verbose(message),
        _ => logger.log_quiet(message),
    }
}