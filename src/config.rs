use crate::logger::{Log, Logger};
use crate::messages::constants::config::{
    BLOCKS_FILE, HEADERS_FILE, LOG_FILE, VERBOSE, QUIET, START_TIMESTAMP, TCP_TIMEOUT, PRIVATE_KEY_FILE
};
use crate::messages::HashId;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    seed: String,
    start_timestamp: u32,
    headers_file: String,
    blocks_file: String,
    tcp_timeout_seconds: u64,
    logger: Logger,
    genesis_hash: HashId,
    private_key_file: String,
}

impl Config {
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

    pub fn log(&self, content: &str, level: &str) {
        let _ = self.logger.log_sender.clone().send(match level {
            VERBOSE => Log::Verbose(content.to_string()),
            _ => Log::Quiet(content.to_string()),
        });
    }

    pub fn get_genesis(&self) -> HashId {
        self.genesis_hash
    }

    pub fn get_private_key_file(&self) -> &str {
        &self.private_key_file
    }

    fn remove_or(hashmap: &mut HashMap<String, String>, key: &str, default: &str) -> String {
        hashmap.remove(key).unwrap_or(default.to_string())
    }

    fn from_hashmap(mut values: HashMap<String, String>) -> io::Result<Config> {
        Ok(Config {
            seed: Config::remove_or(&mut values, "seed", ""),
            start_timestamp: Config::remove_or(&mut values, "start_timestamp", "")
                .parse()
                .unwrap_or(START_TIMESTAMP),
            logger: Logger::new(
                Config::remove_or(&mut values, "log_file", LOG_FILE),
                Config::remove_or(&mut values, "log_level", QUIET),
            ),
            headers_file: Config::remove_or(&mut values, "headers_file", HEADERS_FILE),
            blocks_file: Config::remove_or(&mut values, "blocks_file", BLOCKS_FILE),
            tcp_timeout_seconds: Config::remove_or(&mut values, "tcp_timeout_seconds", "")
                .parse()
                .unwrap_or(TCP_TIMEOUT),
            genesis_hash: Self::hash_from_string(&Config::remove_or(
                &mut values,
                "genesis_hash",
                "",
            ))?,
            private_key_file: Config::remove_or(&mut values, "private_key_file", PRIVATE_KEY_FILE),
        })
    }

    pub fn from_file(path: PathBuf) -> io::Result<Config> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut values = HashMap::new();
        for line in reader.lines() {
            if let Some((key, value)) = line?.split_once('=') {
                values.insert(key.to_owned(), value.to_owned());
            }
        }
        Config::from_hashmap(values)
    }

    fn hash_from_string(string: &str) -> io::Result<HashId> {
        if string.len() != 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Hash length is invalid, expecting 64 characters in hex",
            ));
        }
        let mut bytes = [0u8; 64];
        for (index, c) in string.chars().enumerate() {
            match c.to_digit(16) {
                // hash should be hexadecimal
                Some(byte_value) => bytes[index] = byte_value as u8,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Hash contains non hex characters",
                    ))
                }
            };
        }
        let mut hash = [0u8; 32];
        for i in 0..32 {
            hash[31 - i] = bytes[i * 2] << 4 | bytes[i * 2 + 1];
        }
        Ok(HashId::new(hash))
    }
}
