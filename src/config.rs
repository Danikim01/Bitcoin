use crate::messages::constants::config::{
    BLOCKS_FILE, HEADERS_FILE, LOG_FILE, PORT, QUIET, TCP_TIMEOUT,
};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    seed: String,
    start_timestamp: u32,
    log_level: String,
    log_file: String,
    headers_file: String,
    blocks_file: String,
    tcp_timeout_seconds: u64,
}

impl Config {
    pub fn new(
        seed: String,
        start_timestamp: u32,
        log_level: String,
        log_file: String,
        headers_file: String,
        blocks_file: String,
        tcp_timeout_seconds: u64,
    ) -> Self {
        Self {
            seed,
            start_timestamp,
            log_level,    //
            log_file,     //
            headers_file, //
            blocks_file,  //
            tcp_timeout_seconds,
        }
    }

    pub fn default() -> Self {
        Self::new(
            "".to_owned() + ":" + &format!("{}", PORT),
            1,
            QUIET.to_string(),
            LOG_FILE.to_string(),
            HEADERS_FILE.to_string(),
            BLOCKS_FILE.to_string(),
            TCP_TIMEOUT,
        )
    }

    pub fn get_tcp_timeout(&self) -> u64 {
        self.tcp_timeout_seconds
    }

    pub fn get_start_timestamp(&self) -> u32 {
        self.start_timestamp
    }

    pub fn get_hostname(&self) -> String {
        self.seed.clone()
    }

    pub fn get_log_level(&self) -> String {
        self.log_level.clone()
    }

    pub fn get_log_file(&self) -> &str {
        &self.log_file
    }

    pub fn get_headers_file(&self) -> &str {
        &self.headers_file
    }

    pub fn get_blocks_file(&self) -> &str {
        &self.blocks_file
    }

    pub fn from_file(path: PathBuf) -> Result<Config, io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut config = Config::default();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            match index {
                0 => config.seed = line,
                1 => config.start_timestamp = line.parse().unwrap_or(1681095600),
                2 => config.log_level = line,
                3 => config.log_file = line,
                4 => config.headers_file = line,
                5 => config.blocks_file = line,
                6 => config.tcp_timeout_seconds = line.parse().unwrap_or(TCP_TIMEOUT),
                _ => break,
            }
        }
        Ok(config)
    }
}
