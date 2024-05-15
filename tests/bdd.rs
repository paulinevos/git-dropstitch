use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;

use cucumber::codegen::IntoWorldResult;
use cucumber::gherkin::Scenario;
use cucumber::{given, then, when, World as _};
use fs_extra::dir::CopyOptions;

use gds_lib::dropstitch::Dropstitch;
use gds_lib::git::run_command_with_output;
use gds_lib::{Cli, Command};
use regex::Regex;
use uuid::Uuid;

#[derive(cucumber::World, Debug, Default)]
#[world(init = Self::new)]
struct World {
    path: PathBuf,
}

impl World {
    pub fn new() -> Self {
        let uuid = Uuid::new_v4();

        Self {
            path: PathBuf::from(MOCK_REPO_PATH)
                .parent()
                .unwrap()
                .join(uuid.to_string()),
        }
    }

    fn run_command_with_output(&self, args: &[&str]) -> anyhow::Result<String> {
        run_command_with_output(&args, Some(&self.path))
    }
}
const MOCK_REPO_PATH: &str = "tests/fixtures/mock-repo/";

// TODO: remove this. Doesn't make sense to run async as it's fucking up mock repo state
#[tokio::main]
async fn main() {
    let w = World::new();

    World::cucumber()
        .before(|_feature, _rule, _scenario, world| {
            Box::pin(async move {
                let mock_repo = PathBuf::from(MOCK_REPO_PATH);
                let scenario_repo = world.path.clone();

                let options = CopyOptions::default();
                let options = options.copy_inside(true);

                // Copy mock repo to scenario repo
                fs_extra::copy_items(&[mock_repo], scenario_repo.clone(), &options).unwrap();

                // Copy over /git directory in mock repo to .git, so it will actually act like a repo.
                fs_extra::copy_items(
                    &[scenario_repo.join("git").to_str().unwrap()],
                    scenario_repo.join(".git").to_str().unwrap(),
                    &options,
                )
                .unwrap();
            })
        })
        .after(|_feature, _rule, _scenario, _ev, world| {
            Box::pin(async {
                fs::remove_dir_all(world.unwrap().path.clone())
                    .expect("Failed to remove test repo");
            })
        })
        .run("tests/features/undo.feature")
        .await;
}

#[given(expr = "the user amended the latest commit message from \"foo\" to {string}")]
async fn amended_commit(w: &mut World, to: String) -> anyhow::Result<()> {
    let _ = w.run_command_with_output(&["checkout", "amend-amend"])?;

    commit_message_is(w, to).await
}

#[when("they undo the last change")]
async fn undo_last(w: &mut World) -> anyhow::Result<()> {
    Dropstitch::run(Cli::init(Command::Z, w.path.clone()))?;
    Ok(())
}

#[then(expr = "the latest commit message is {string}")]
async fn commit_message_is(w: &mut World, msg: String) -> anyhow::Result<()> {
    let output = w.run_command_with_output(&["log", "--oneline"])?;
    let re = Regex::new(format!("^[a-z0-9]* {}\n", msg).as_str())?;

    assert!(re.is_match(output.as_str()));

    Ok(())
}
