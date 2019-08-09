//! Print to stdout a new maintainer list Nix file, with as many IDs
//! filled in as possible.

#![warn(missing_docs)]

use crate::filemunge;
use crate::maintainers::{GitHubID, GitHubName, MaintainerList};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;
use tokio::runtime::Runtime;

pub fn backfill_ids(
    logger: slog::Logger,
    users: hubcaps::users::Users,
    file: &Path,
    maintainers: MaintainerList,
) {
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
