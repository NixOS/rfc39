//! bPrint to stdout a new maintainer list Nix file, with as many IDs
//! filled in as possible.

#![warn(missing_docs)]

use crate::filemunge;
use crate::maintainerhistory::{Confidence, MaintainerHistory};
use crate::maintainers::{GitHubID, GitHubName, MaintainerList};
use hubcaps::Github;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;
use tokio::runtime::Runtime;

pub fn backfill_ids(
    logger: slog::Logger,
    github: Github,
    file: &Path,
    maintainers: MaintainerList,
) {
    let mut rt = Runtime::new().unwrap();

    let missing_ids = maintainers
        .into_iter()
        .filter(|(_handle, maintainer)| {
            maintainer.github.is_some() && maintainer.github_id.is_none()
        })
        .map(|(handle, maintainer)| {
            (
                maintainer
                    .github
                    .clone()
                    .expect("should be safe because of prior filter"),
                maintainer,
                handle,
            )
        });

    info!(logger, "Loading the maintainer list's GitHub accounts and blame history";
          "commit" => "");

    let history = MaintainerHistory::load(logger.clone(), file);

    info!(logger, "Loaded the maintainer list's GitHub accounts and blame history";
          "commit" => "");

    let found_ids: HashMap<GitHubName, GitHubID> = missing_ids
        .filter_map(|(github_name, maintainer, handle)| {
            debug!(logger, "Getting ID for user";
                  "github_account" => %github_name,
            );

            match rt.block_on(github.users().get(github_name.to_string())) {
                Ok(user) => {
                    debug!(logger, "Found ID for user";
                          "github_account" => %github_name,
                          "id" => %user.id);
                    Some((github_name, maintainer, GitHubID::new(user.id), handle))
                }
                Err(e) => {
                    warn!(logger, "Error fetching ID for user";
                          "github_account" => %github_name,
                          "e" => %e);
                    None
                }
            }
        })
        .filter_map(|(github_name, _maintainer, github_id, handle)| {
            let confidence =
                history.confidence_for_user(&github, &handle, &github_name, github_id)?;

            if confidence == Confidence::Total {
                Some((github_name, github_id))
            } else {
                info!(logger,
                      "Non-total confidence for user";
                      "confidence" => %format!("{:#?}", confidence),
                      "user" => %handle,
                );
                None
            }
        })
        .collect();

    println!(
        "{}",
        filemunge::backfill_file(found_ids, read_to_string(file).unwrap(),)
    );
}
