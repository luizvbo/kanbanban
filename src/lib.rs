pub mod app;
pub mod events;
pub mod types;
pub mod ui;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        // Test default
        let args = Args::parse_from(["kanbanban"]);
        assert_eq!(args.path, None);

        // Test custom path
        let args = Args::parse_from(["kanbanban", "my_board.yaml"]);
        assert_eq!(args.path, Some(PathBuf::from("my_board.yaml")));
    }
}
