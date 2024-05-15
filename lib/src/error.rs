use cucumber::codegen::anyhow;
use thiserror::Error;

use crate::Action;

#[derive(Error, Debug)]
pub enum DropstitchError {
    #[error("Dropstitch must be run from inside a Git repository")]
    NotAGitRepository,
    #[error("Error: nothing to {0}")]
    NothingTo(Action),
    #[error("Dropstitch can't be run from detached HEAD state")]
    DetachedHead,
    #[error("Dropstitch can't be run while {0} is in progress")]
    OperationInProgress(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl DropstitchError {
    pub fn into_exit_code(self) -> i32 {
        match self {
            DropstitchError::NotAGitRepository => 2,
            DropstitchError::NothingTo(_) => 2,
            DropstitchError::DetachedHead => 2,
            DropstitchError::OperationInProgress(_) => 2,
            _ => 1,
        }
    }

    pub fn display_for_user(&self) {
        match self {
            DropstitchError::NotAGitRepository => println!("{}", self),
            DropstitchError::NothingTo(_) => println!("{}", self),
            DropstitchError::DetachedHead => {
                println!("You appear to be in a detached HEAD state.\n");
                println!(
                    "Dropstitch must be run from the \"end\" (HEAD) of a branch.\n
                Try checking out the branch with `git switch [branch name]`"
                );
            }
            DropstitchError::OperationInProgress(op) => {
                // ToDo: make a nice enum for the ops
                if op == "rebasing" {
                    println!("You appear to be in an active rebase.\n");
                    println!(
                        "Dropstitch must be run from the \"end\" (HEAD) of a branch.\n
                    Please finish rebasing, or leave it behind with `git rebase --quit`"
                    );
                } else if op == "bisect" {
                    println!("You appear to be in an active bisect.\n");
                    println!(
                        "Dropstitch must be run from the \"end\" (HEAD) of a branch.\n
                    Please finish your bisect, or leave it behind with `git bisect --reset`"
                    );
                } else {
                    println!("You appear to be in some sort of active Git operation.\n");
                    println!(
                        "Dropstitch must be run from the \"end\" (HEAD) of a branch.\n
                    Please finish the operation to return to your head."
                    );
                }
            }
            // ToDo: not be an asshole and print a more useful error
            // ToDo: Also run in --verbose mode to get internal error
            e => println!("An internal error occurred. My bad! {}", e),
        }
    }
}
