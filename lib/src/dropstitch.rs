use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use regex::Regex;
use strum::EnumString;

use crate::error::DropstitchError;
use crate::git::run_command_with_output;
use crate::{Action, Cli, Command};

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

#[derive(Debug)]
struct Ref {
    from_hash: String,
    to_hash: String,
    /// The Git operation to perform the action onto, i.e. merge or rebase
    operation: Operation,
}

#[derive(EnumString, Debug, PartialEq)]
#[strum(serialize_all = "snake_case")]
enum Operation {
    Amend,
    //     Merge,
    //     Reset { hard: bool },
    //     Rebase {interactive: bool, onto: String},
}

impl Ref {
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

struct Reflog {
    refs: Vec<Ref>,
    repo: PathBuf,
}

impl Reflog {
    pub fn init(repo: Option<PathBuf>) -> Result<Reflog, DropstitchError> {
        let repo = if let Some(repo) = repo {
            repo
        } else {
            PathBuf::new()
        };

        let branch = Self::parse_branch(&repo)?;

        let reflog_path = repo.join(".git").join("logs/refs/heads").join(branch);

        let file = File::open(reflog_path)?;
        let lines: anyhow::Result<Vec<Option<Ref>>> = BufReader::new(file)
            .lines()
            .map(|l| -> anyhow::Result<Option<Ref>> { Ok(Ref::from_line(l?)?) })
            .collect();

        let refs: Vec<Ref> = lines?
            .into_iter()
            .filter(|l| l.is_some())
            .map(|l| l.unwrap())
            .collect::<Vec<Ref>>();

        Ok(Self { refs, repo })
    }
    pub fn undo_previous(&self) -> Result<(), DropstitchError> {
        let prev = self.refs.last();

        // Reset (if this fails, don't write to file)
        // Write to dropstitch file

        self.reset_to_ref(Action::Undo, &prev)
    }

    pub fn redo_next(&self) -> Result<(), DropstitchError> {
        // Reflog::reset_to_ref(Action::Redo, &r)
        Ok(())
    }

    fn reset_to_ref(
        &self,
        action: Action,
        ref_option: &Option<&Ref>,
    ) -> Result<(), DropstitchError> {
        if let Some(r) = ref_option {
            run_command_with_output(&["reset", "--hard", r.from_hash.as_str()], Some(&self.repo))?;
            Ok(())
        } else {
            Err(DropstitchError::NothingTo(action))
        }
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

        if let Some(_) = caps.name("detached") {
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
