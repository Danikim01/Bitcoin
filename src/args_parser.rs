use std::env;
use std::fs::canonicalize;
use std::path::{Path, PathBuf};
use std::process::exit;

fn help() {
    eprintln!(
        "Usage:
`$ ./nodo-rustico /path/to/nodo.conf`
"
    );
}

pub fn get_args() -> PathBuf {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        // no arguments passed
        1 => {
            eprintln!("Error: Config file not provided");
            help();
            exit(1);
        }
        2 => match canonicalize(Path::new(&args[1])) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Error: Couldn't resolve path to config file. {e}");
                exit(2);
            }
        },
        _ => {
            eprintln!("Error: Too many arguments, only one was expected.");
            help();
            exit(1);
        }
    }
}
