use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod dropstitch;
pub mod error;

pub mod git {
    use anyhow::Context;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    pub fn run_command_with_output(
        args: &[&str],
        path: Option<&PathBuf>,
    ) -> anyhow::Result<String> {
        let mut child = Command::new("git");

        if let Some(path) = path {
            child.args(["-C", path.to_str().context("Failed parsing repo path")?]);
        }

        for arg in args {
            child.arg(arg);
        }

        let output = child.stdout(Stdio::piped()).spawn()?.wait_with_output()?;
        let output = std::str::from_utf8(&output.stdout)?;

        Ok(String::from(output))
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
/// Undo or redo any Git operation you thought was permanent
pub struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(global = true)]
    path: Option<PathBuf>,
}

impl Cli {
    pub fn init(command: Command, path: PathBuf) -> Self {
        Self {
            command,
            path: Some(path),
        }
    }
}

#[derive(Subcommand, Debug, Default)]
pub enum Command {
    /// Undo last
    Z,
    /// Redo next
    Y,
    /// List available actions (default)
    #[default]
    Ls,
}
