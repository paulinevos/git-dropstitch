use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::{Display, EnumString};

use crate::error::DropstitchError;
use crate::git::run_command_with_output;
use crate::{Cli, Command};

const HEAD_REGEX: &str = r"^\* ((?<detached>\(HEAD detached)|(?<operation>\(no branch, (bisect|rebasing))|(?<branch>.+))";
const REF_REGEX: &str =
    r"^(?<from_hash>[[:alnum:]]{40}) (?<to_hash>[[:alnum:]]{40}) .+\((?<op>amend)\): .+$";

pub struct Dropstitch;

impl Dropstitch {
    pub fn run(cli: Cli) -> Result<(), DropstitchError> {
        let reflog = Reflog::init(cli.path)?;

        match &cli.command {
            Command::Z => reflog.undo_previous(),
            Command::Y => reflog.redo_next(),
            Command::Ls => Ok(println!("Printing list")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Ref {
    from_hash: String,
    to_hash: String,
    /// The Git operation to perform the action onto, i.e. merge or rebase
    operation: Operation,
}

#[derive(EnumString, Debug, PartialEq, Serialize, Deserialize, Clone)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "lowercase")]
enum Operation {
    Amend,
    //     Merge,
    //     Reset { hard: bool },
    //     Rebase {interactive: bool, onto: String},
}

impl FromLine for Ref {
    fn from_line(l: String) -> anyhow::Result<Option<Self>> {
        let re = Regex::new(REF_REGEX)?;

        let mut it = re.captures_iter(l.as_str());
        if let Some(caps) = it.next() {
            return Ok(Some(Ref {
                from_hash: caps
                    .name("from_hash")
                    .context("missing 'from' hash")?
                    .as_str()
                    .to_string(),
                to_hash: caps
                    .name("to_hash")
                    .context("missing 'to' hash")?
                    .as_str()
                    .to_string(),
                operation: Operation::from_str(
                    caps.name("op").context("missing operation")?.as_str(),
                )?,
            }));
        }

        Ok(None)
    }
}

#[derive(EnumString, Display, Debug, Serialize, Deserialize, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum Action {
    Undo,
    Redo,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ActionPerformed {
    action: Action,
    reference: Ref,
}

impl FromLine for ActionPerformed {
    fn from_line(s: String) -> anyhow::Result<Option<Self>> {
        Ok(serde_json::from_str(s.as_str())?)
    }
}

struct Reflog {
    refs: Vec<Ref>,
    action_log: Vec<ActionPerformed>,
    repo_path: PathBuf,
    dropstitch_file: File,
}

trait FromLine {
    fn from_line(s: String) -> anyhow::Result<Option<Self>>
    where
        Self: Sized;
}

impl Reflog {
    pub fn init(repo_path: Option<PathBuf>) -> Result<Reflog, DropstitchError> {
        let repo_path = if let Some(path) = repo_path {
            path
        } else {
            PathBuf::new()
        };

        let branch = Self::parse_branch(&repo_path)?;
        let git_dir = repo_path.join(".git");

        let dropstitch_path = git_dir.join(".dropstitch");
        create_dir_all(&dropstitch_path)?;

        let dropstitch_file = Self::dropstitch_file(dropstitch_path.join(&branch))?;

        Ok(Self {
            refs: Self::parse_refs(&git_dir, &branch)?,
            action_log: Self::parse_actions(&dropstitch_file)?,
            repo_path,
            dropstitch_file,
        })
    }

    fn parse_refs(git_dir: &Path, branch: &str) -> Result<Vec<Ref>, DropstitchError> {
        let reflog_path = git_dir.join("logs/refs/heads").join(branch);
        let reflog_file = File::open(reflog_path)?;

        Ok(Self::parse_file_into(&reflog_file)?)
    }

    fn parse_actions(dropstitch_file: &File) -> Result<Vec<ActionPerformed>, DropstitchError> {
        Ok(Self::parse_file_into(dropstitch_file)?)
    }

    // ToDo: make a version of this that stops reading lines when the first undo/redo is found (Iterator::take_while?)
    fn parse_file_into<T: FromLine>(file: &File) -> anyhow::Result<Vec<T>> {
        let lines: anyhow::Result<Vec<Option<T>>> = BufReader::new(file)
            .lines()
            .map(|l| -> anyhow::Result<Option<T>> {
                let l = l?;
                T::from_line(l)
            })
            .collect();

        Ok(lines?.into_iter().flatten().collect())
    }

    fn dropstitch_file(dropstitch_path: PathBuf) -> Result<File, DropstitchError> {
        Ok(OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(dropstitch_path)?)
    }

    fn get_prev(&self) -> Option<Ref> {
        for r in self.refs.clone().iter().rev() {
            if !self.action_log.contains(&ActionPerformed {
                action: Action::Undo,
                reference: r.clone(),
            }) {
                return Some(r.clone());
            }
        }
        None
    }

    pub fn undo_previous(&self) -> Result<(), DropstitchError> {
        self.perform_action(Action::Undo, self.get_prev())
    }

    pub fn redo_next(&self) -> Result<(), DropstitchError> {
        // Reflog::reset_to_ref(Action::Redo, &r)
        Ok(())
    }

    fn perform_action(
        &self,
        action: Action,
        ref_option: Option<Ref>,
    ) -> Result<(), DropstitchError> {
        if let Some(reference) = ref_option {
            run_command_with_output(
                &["reset", "--hard", reference.from_hash.as_str()],
                Some(&self.repo_path),
            )?;

            writeln!(
                &self.dropstitch_file,
                "{}",
                json!(ActionPerformed { action, reference })
            )?;
        } else {
            return Err(DropstitchError::NothingTo(action));
        }

        Ok(())
    }

    fn parse_branch(repo: &PathBuf) -> Result<String, DropstitchError> {
        let head = repo.join(".git/HEAD");

        if !head.exists() || !head.is_file() {
            return Err(DropstitchError::NotAGitRepository);
        }

        // ToDo: maybe just fail here instead of doing manual validation
        // ToDo: Pass the repo path instead of the HEAD path
        let branch_output = run_command_with_output(&["branch"], Some(repo))?;

        let re = Regex::new(HEAD_REGEX).context("regex failed")?;
        let mut it = re.captures_iter(branch_output.as_str());
        let caps = it.next().context("regex capture failed")?;

        if caps.name("detached").is_some() {
            return Err(DropstitchError::DetachedHead);
        }

        if let Some(op) = caps.name("operation") {
            return Err(DropstitchError::OperationInProgress(String::from(
                op.as_str(),
            )));
        }

        if let Some(branch) = caps.name("branch") {
            return Ok(String::from(branch.as_str()));
        }

        Err(anyhow::format_err!("could not determine HEAD state"))?
    }
}
