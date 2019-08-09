//! Compare and sync maintainers from Nixpkgs to maintainers on
//! GitHub Maintainer team, as described in RFC #39:
//! https://github.com/NixOS/rfcs/blob/master/rfcs/0039-unprivileged-maintainer-teams.md

#![warn(missing_docs)]

#[macro_use]
extern crate slog;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::path::PathBuf;
use structopt::StructOpt;
mod maintainers;
use maintainers::{GitHubID, GitHubName, MaintainerList};
mod filemunge;
use hubcaps::{Credentials, Github};
use hyper::client::connect::Connect;
use std::env;
use std::fs::read_to_string;
use std::path::Path;
use tokio::runtime::Runtime;

#[derive(Debug, StructOpt)]
struct Options {
    /// Maintainer list
    #[structopt(short = "m", long = "maintainers", parse(from_os_str))]
    maintainers: PathBuf,

    /// Execution Mode
    #[structopt(subcommand)]
    mode: ExecMode,
}

#[derive(Debug, StructOpt)]
enum ExecMode {
    /// Verify maintainers, their GitHub handle, and GitHub ID
    #[structopt(name = "check-handles")]
    CheckHandles,

    /// Poorly edit the maintainers.nix file to add missing GitHub IDs
    #[structopt(name = "backfill-ids")]
    BackfillIDs,
}

fn main() {
    let logger = rfc39::default_logger();

    let inputs = Options::from_args();

    let maintainers_file = inputs.maintainers.canonicalize().unwrap();
    info!(logger, "Loading maintainer information";
          "from" => inputs.maintainers.display(),
          "absolute" => maintainers_file.display()
    );

    let maintainers = MaintainerList::load(logger.clone(), &maintainers_file).unwrap();

    let github = Github::new(
        String::from("NixOS/rfcs#39 (hubcaps)"),
        env::var("GITHUB_TOKEN").ok().map(Credentials::Token),
    );

    match inputs.mode {
        ExecMode::CheckHandles => {
            check_handles(logger.new(o!("exec-mode" => "CheckHandles")), maintainers)
        }
        ExecMode::BackfillIDs => backfill_ids(
            logger.new(o!("exec-mode" => "BackfillIDs")),
            github.users(),
            &maintainers_file,
            maintainers,
        ),
    }
}

fn backfill_ids<T>(
    logger: slog::Logger,
    users: hubcaps::users::Users<T>,
    file: &Path,
    maintainers: MaintainerList,
) where
    T: Clone + Connect + 'static,
{
    let mut rt = Runtime::new().unwrap();

    let missing_ids = maintainers
        .into_iter()
        .filter(|(_, maintainer)| maintainer.github.is_some() && maintainer.github_id.is_none())
        .map(|(_, maintainer)| {
            maintainer
                .github
                .expect("should be safe because of prior filter")
        });

    let found_ids: HashMap<GitHubName, GitHubID> = missing_ids
        .filter_map(|github_name| {
            info!(logger, "Getting ID for user";
                  "github_account" => %github_name,
            );

            match rt.block_on(users.get(github_name.to_string())) {
                Ok(user) => {
                    info!(logger, "Found ID for user";
                          "github_account" => %github_name,
                          "id" => %user.id);
                    Some((github_name, GitHubID::new(user.id)))
                }
                Err(e) => {
                    warn!(logger, "Error fetching ID for user";
                          "github_account" => %github_name,
                          "e" => %e);
                    None
                }
            }
        })
        .collect();

    println!(
        "{}",
        filemunge::backfill_file(found_ids, read_to_string(file).unwrap(),)
    );
}

fn check_handles(logger: slog::Logger, maintainers: MaintainerList) {
    for (handle, info) in maintainers {
        match (info.github, info.github_id) {
            (Some(name), Some(id)) => {
                info!(logger, "todo: check if ID is up to date";
                      "github_account" => %name,
                      "github_id" => %id,
                );
            }
            (Some(name), None) => {
                warn!(logger, "Missing GitHub ID";
                       "github_account" => %name);
            }
            (None, Some(id)) => {
                error!(logger, "Missing GitHub Account, but ID present";
                       "who" => %handle,
                       "github_id" => %id,
                );
            }
            (None, None) => {
                debug!(logger, "Missing GitHub Account and ID";
                       "who" => %handle);
            }
        }
    }
}
