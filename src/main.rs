//! Compare and sync maintainers from Nixpkgs to maintainers on
//! GitHub Maintainer team, as described in RFC #39:
//! https://github.com/NixOS/rfcs/blob/master/rfcs/0039-unprivileged-maintainer-teams.md

#![warn(missing_docs)]

#[macro_use]
extern crate slog;

use std::path::PathBuf;
use structopt::StructOpt;
mod maintainers;
use maintainers::MaintainerList;

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

    match inputs.mode {
        ExecMode::CheckHandles => {
            check_handles(logger.new(o!("exec-mode" => "CheckHandles")), maintainers)
        }
    }
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
