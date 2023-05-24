use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

struct Logger {
    log_file: Arc<Mutex<std::fs::File>>,
}

impl Logger {
    fn new() -> Logger {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("log.txt")?;
        Logger {
            log_file: Arc::new(Mutex::new(file)),
        }
    }

    fn log(&self, message: &str) {
        let line = format!("{}\n", message);
        print!("{}", line);
        let mut file = self.log_file.lock().unwrap();
        file.write_all(line.as_bytes())?;
    }
}
