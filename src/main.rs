//! Compare and sync maintainers from Nixpkgs to maintainers on
//! GitHub Maintainer team, as described in RFC #39:
//! https://github.com/NixOS/rfcs/blob/master/rfcs/0039-unprivileged-maintainer-teams.md

#![warn(missing_docs)]

#[macro_use]
extern crate slog;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use structopt::StructOpt;
mod cli;
use cli::{ExecMode, Options};
mod maintainers;
use maintainers::MaintainerList;
mod filemunge;
mod maintainerhistory;
mod nix;
mod op_backfill;
mod op_blame_author;
mod op_check_handles;
mod op_sync_team;
use hubcaps::{Credentials, Github, InstallationTokenGenerator, JWTCredentials};

/// Github Authentication information for the GitHub app.
/// When creating the application, the only permission it needs
/// is Members: Read and Write.
/// No access to code or other permissions is needed.
// NOTE: DO NOT MAKE "Debug"! This will leak secrets
#[derive(Deserialize)]
pub struct GitHubAuth {
    /// Overall GitHub Application ID, same for all users
    pub app_id: u64,

    /// DER RSA key. Generate with
    /// `openssl rsa -in private_rsa_key.pem -outform DER -out private_rsa_key.der`
    pub private_key_file: PathBuf,

    /// the ID of the installation of this app in to the repo or
    /// organization.
    pub installation_id: u64,
}

fn main() {
    let (logger, _scopes) = rfc39::default_logger();

    let inputs = Options::from_args();

    let maintainers_file = inputs.maintainers.canonicalize().unwrap();
    info!(logger, "Loading maintainer information";
          "from" => inputs.maintainers.display(),
          "absolute" => maintainers_file.display()
    );

    let maintainers = MaintainerList::load(logger.clone(), &maintainers_file).unwrap();

    let github_auth = nix::nix_instantiate_file_to_struct::<GitHubAuth>(
        logger.new(o!()),
        &inputs.credential_file,
    )
    .expect("Failed to parse the credential file");

    let mut private_key = Vec::new();
    File::open(&github_auth.private_key_file)
        .expect("Opening the private key file")
        .read_to_end(&mut private_key)
        .expect("Reading the private key");

    let github = Github::new(
        String::from("NixOS/rfcs#39 (hubcaps)"),
        Credentials::InstallationToken(InstallationTokenGenerator::new(
            github_auth.installation_id,
            JWTCredentials::new(github_auth.app_id, private_key).unwrap(),
        )),
    )
    .unwrap();

    match inputs.mode {
        ExecMode::CheckHandles => op_check_handles::check_handles(
            logger.new(o!("exec-mode" => "CheckHandles")),
            maintainers,
        ),
        ExecMode::BackfillIDs => op_backfill::backfill_ids(
            logger.new(o!("exec-mode" => "BackfillIDs")),
            github,
            &maintainers_file,
            maintainers,
        ),
        ExecMode::BlameAuthor => op_blame_author::report(
            logger.new(o!("exec-mode" => "BlameAuthor")),
            github,
            &maintainers_file,
            maintainers,
        ),
        ExecMode::SyncTeam(team_info) => op_sync_team::sync_team(
            logger.new(o!("exec-mode" => "SyncTeam")),
            github,
            maintainers,
            &team_info.organization,
            team_info.team_id,
            team_info.dry_run,
            team_info.limit,
        ),
        ExecMode::ListTeams(team_info) => op_sync_team::list_teams(github, &team_info.organization),
    }
}
