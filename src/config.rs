use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use crate::messages::constants::config::PATH;

pub struct Config {
    seed: String,
    port: String,
    start_date: String,
}

impl Config {
    pub fn new(seed: String, port: String, start_date: String) -> Config {
        Config {
            seed,
            port,
            start_date,
        }
    }

    pub fn default() -> Config {
        Config {
            seed: "".to_string(),
            port: "".to_string(),
            start_date: "".to_string(),
        }
    }

    pub fn get_seed(&self) -> &String {
        &self.seed
    }

    pub fn get_port(&self) -> &String {
        &self.port
    }

    pub fn get_start_date(&self) -> &String {
        &self.start_date
    }

    pub fn get_hostname(&self) -> String {
        self.seed.to_owned() + ":" + &self.port
    }

    pub fn from_file() -> Result<Config, io::Error> {
        let file = File::open(PATH)?;
        let reader = BufReader::new(file);

        let mut config = Config::default();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            match index {
                0 => config.seed = line,
                1 => config.port = line,
                2 => config.start_date = line,
                _ => (),
            }
        }

        Ok(config)
    }
}