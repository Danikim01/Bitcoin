use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex, Once};

struct Logger {
    log_file: Arc<Mutex<std::fs::File>>,
}

impl Logger {
    fn new() -> Logger {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("src/log.txt").expect("Failed to open log file");
        Logger {
            log_file: Arc::new(Mutex::new(file)),
        }
    }

    fn log(&self, message: &str) {
        let line = format!("{}\n", message);
        print!("{}", line);
        let mut file = self.log_file.lock().unwrap();
        file.write_all(line.as_bytes()).expect("Failed to write to log file");
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

pub fn log(message: &str) {
    let mut lazy_logger = LazyLogger::new();
    let logger = lazy_logger.get_logger();
    logger.log(message);
}