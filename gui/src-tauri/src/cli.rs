use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Customize folder icons")]
struct Cli {
    /// Folders to list in the app on launch.
    #[arg(value_name = "DIR")]
    directories: Vec<PathBuf>,
}

/// Folders passed on the command line, as absolute paths.
pub struct LaunchDirectories(pub Vec<PathBuf>);

impl LaunchDirectories {
    /// Parses the process arguments, exiting on `--help`, `--version` or bad usage.
    ///
    /// Arguments that aren't existing folders are skipped with a warning rather than
    /// aborting, so a stale shortcut or file-manager "Open with" still opens the app.
    pub fn from_args() -> Self {
        let mut directories = Vec::new();
        for path in Cli::parse().directories {
            match std::path::absolute(&path) {
                Ok(abs) if abs.is_dir() => {
                    if !directories.contains(&abs) {
                        directories.push(abs);
                    }
                }
                _ => eprintln!("Ignoring {}: not a folder", path.display()),
            }
        }
        Self(directories)
    }
}
