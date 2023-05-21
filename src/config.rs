use crate::messages::constants::config::PATH;
use crate::messages::constants::config::PORT;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};

pub struct Config {
    seed: String,
    port: u16,
    start_timestamp: u32,
}

impl Config {
    pub fn new(seed: String, port: u16, start_timestamp: u32) -> Config {
        Config {
            seed,
            port,
            start_timestamp,
        }
    }

    pub fn default() -> Config {
        Config {
            seed: "".to_string(),
            port: "".to_string().parse().unwrap_or(PORT),
            start_timestamp: 1,
        }
    }

    pub fn get_seed(&self) -> &String {
        &self.seed
    }

    pub fn get_port(&self) -> &u16 {
        &self.port
    }

    pub fn get_start_timestamp(&self) -> u32 {
        self.start_timestamp
    }

    pub fn get_hostname(&self) -> String {
        self.seed.to_owned() + ":" + &self.port.to_string()
    }

    pub fn from_file() -> Result<Config, io::Error> {
        let file = File::open(PATH)?;
        let reader = BufReader::new(file);

        let mut config = Config::default();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            match index {
                0 => config.seed = line,
                1 => config.port = line.parse().unwrap_or(PORT),
                2 => config.start_timestamp = line.parse().unwrap_or(1681095600),
                _ => (),
            }
        }

        Ok(config)
    }
}
