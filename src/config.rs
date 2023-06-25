use crate::messages::constants::config::{
    BLOCKS_FILE, HEADERS_FILE, LOG_FILE, PORT, QUIET, TCP_TIMEOUT, START_TIMESTAMP
};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use crate::logger::Logger;

#[derive(Clone)]
pub struct Config {
    seed: String,
    start_timestamp: u32,
    headers_file: String,
    blocks_file: String,
    tcp_timeout_seconds: u64,
    logger: Logger,
}

impl Config {
    pub fn new(
        seed: String,
        start_timestamp: u32,
        headers_file: String,
        blocks_file: String,
        tcp_timeout_seconds: u64,
        logger: Logger
    ) -> Self {
        Self {
            seed,
            start_timestamp,
            headers_file,
            blocks_file,
            tcp_timeout_seconds,
            logger
        }
    }

    pub fn default() -> Self {
        Self::new(
            "".to_owned() + ":" + &format!("{}", PORT),
            1,
            HEADERS_FILE.to_string(),
            BLOCKS_FILE.to_string(),
            TCP_TIMEOUT,
            Logger::default()
        )
    }

    pub fn get_tcp_timeout(&self) -> u64 {
        self.tcp_timeout_seconds
    }

    /// Returns the start timestamp for sync
    pub fn get_start_timestamp(&self) -> u32 {
        self.start_timestamp
    }

    pub fn get_hostname(&self) -> &str {
        &self.seed
    }

    pub fn get_headers_file(&self) -> &str {
        &self.headers_file
    }

    pub fn get_blocks_file(&self) -> &str {
        &self.blocks_file
    }

    pub fn get_logger(&self) -> &Logger {
        &self.logger
    }

    pub fn from_file(path: PathBuf) -> Result<Config, io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut config = Config::default();
        let mut log_file = LOG_FILE.to_owned();
        let mut log_level = QUIET.to_owned();
        for line in reader.lines() {
            if let Some((key, value)) = line?.split_once('=') {
                match key {
                    "seed" => config.seed = value.to_owned(),
                    "start_timestamp" => config.start_timestamp = value.parse().unwrap_or(START_TIMESTAMP),
                    "log_file" => log_file = value.to_owned(),
                    "log_level" => log_level = value.to_owned(),
                    "headers_file" => config.headers_file = value.to_owned(),
                    "blocks_file" => config.blocks_file = value.to_owned(),
                    "tcp_timeout_seconds" => config.tcp_timeout_seconds = value.parse().unwrap_or(TCP_TIMEOUT),
                    _ => continue,
                }
            }
        }
        config.logger = Logger::new(log_file, log_level);
        Ok(config)
    }
}
