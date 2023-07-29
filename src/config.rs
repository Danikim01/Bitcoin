use crate::logger::{Log, Logger};
use crate::messages::constants::config::{
    BLOCKS_FILE, HEADERS_FILE, LOG_FILE, PORT, QUIET, START_TIMESTAMP, TCP_TIMEOUT, VERBOSE,
};
use crate::messages::HashId;
use crate::utility::get_parent_path;
use crate::wallet::Wallet;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    seed: String,
    port: u16,
    start_timestamp: u32,
    headers_file: String,
    blocks_file: String,
    tcp_timeout_seconds: u64,
    logger: Logger,
    genesis_hash: HashId,
    wallets_dir: String,
    default_wallet_addr: String,
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

    pub fn get_listening_port(&self) -> u16 {
        self.port
    }

    pub fn get_wallets_dir(&self) -> &str {
        &self.wallets_dir
    }

    pub fn get_default_wallet_addr(&self) -> &str {
        &self.default_wallet_addr
    }

    fn remove_or(hashmap: &mut HashMap<String, String>, key: &str, default: &str) -> String {
        hashmap.remove(key).unwrap_or(default.to_string())
    }

    pub fn wallet_from_file(secret_key_file: String) -> io::Result<Option<Wallet>> {
        match fs::read_to_string(&secret_key_file) {
            Ok(file_content) => Ok(Some(file_content.as_str().try_into()?)),
            Err(_e) => {
                let err_msg = format!("Could not read secret key file {}", secret_key_file);
                Err(io::Error::new(io::ErrorKind::Other, err_msg))
            }
        }
    }

    fn from_hashmap(mut values: HashMap<String, String>) -> io::Result<Config> {
        Ok(Config {
            seed: Config::remove_or(&mut values, "seed", ""),
            port: u16::from_str_radix(&Config::remove_or(&mut values, "listening_port", ""), 10)
                .unwrap_or(PORT),
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
            wallets_dir: Config::remove_or(&mut values, "wallets_dir", ""),
            default_wallet_addr: Config::remove_or(&mut values, "default_wallet_addr", ""),
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

        let log_path = PathBuf::from(&values["log_file"]);
        fs::create_dir_all(get_parent_path(log_path))?;
        let config = Config::from_hashmap(values)?;
        let wallet_path = PathBuf::from(config.get_wallets_dir());
        fs::create_dir_all(get_parent_path(wallet_path))?;
        Ok(config)
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
