//! Somewhat half-hearted attempt at checking all the handles and IDs,
//! but it doesn't really work right now.

use crate::maintainers::MaintainerList;

pub fn check_handles(logger: slog::Logger, maintainers: MaintainerList) {
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
