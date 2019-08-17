use crate::maintainerhistory::MaintainerHistory;
use crate::maintainers::MaintainerList;
use hubcaps::Github;
use std::path::Path;

pub fn report(
    logger: slog::Logger,
    github: Github,
    maintainer_file: &Path,
    maintainers: MaintainerList,
) {
    info!(logger, "Verifying our maintainer list GitHub accounts match the author of the commit which added the maintainer entry";
          "commit" => "");

    let history = MaintainerHistory::load(logger.clone(), maintainer_file);

    for (user, information) in maintainers {
        if let Some(github_name) = information.github {
            if let Some(github_id) = information.github_id {
                history.confidence_for_user(&github, &user, &github_name, &github_id);
            }
        }
    }
}
