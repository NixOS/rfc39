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

#[macro_use]
extern crate prometheus;

use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
mod cli;
use cli::{ExecMode, ExitError, Options};
mod invited;
mod maintainers;
use maintainers::MaintainerList;
mod filemunge;
mod maintainerhistory;
mod metrics;
mod nix;
mod op_backfill;
mod op_blame_author;
mod op_check_handles;
mod op_sync_team;
use hubcaps::{Credentials, Github, InstallationTokenGenerator, JWTCredentials};
use prometheus::Encoder;
use std::thread;
use std::time;

/// Github Authentication information for the GitHub app.
/// When creating the application, the only permission it needs
/// is Members: Read and Write.
/// No access to code or other permissions is needed.
// NOTE: DO NOT MAKE "Debug"! This will leak secrets
#[derive(Deserialize)]
pub struct GitHubAppAuth {
    /// Overall GitHub Application ID, same for all users
    pub app_id: u64,

    /// DER RSA key. Generate with
    /// `openssl rsa -in private_rsa_key.pem -outform DER -out private_rsa_key.der`
    pub private_key_file: PathBuf,

    /// the ID of the installation of this app in to the repo or
    /// organization.
    pub installation_id: u64,
}

/// Use a Personal Access Token to run the `blame` and `check` and
/// `backfill`. A GitHubAppAuth must be used for the actual syncing.
/// Needs NO permissions.
#[derive(Deserialize)]
pub struct GitHubTokenAuth {
    /// Personal Access Token
    pub access_token: String,
}

fn load_maintainer_file(logger: slog::Logger, src: &Path) -> Result<MaintainerList, ExitError> {
    let maintainers_file = src.canonicalize()?;

    info!(logger, "Loading maintainer information";
          "from" => src.display(),
          "absolute" => maintainers_file.display()
    );

    Ok(MaintainerList::load(logger.clone(), &maintainers_file)?)
}

fn gh_client_from_args(logger: slog::Logger, credential_file: &Path) -> Github {
    info!(
        logger,
        "Loading GitHub authentication information from {:?}", &credential_file
    );

    let app_auth_load_err: serde_json::error::Error;
    match nix::nix_instantiate_file_to_struct::<GitHubAppAuth>(logger.new(o!()), credential_file) {
        Ok(app_auth) => {
            debug!(logger, "Credential file is providing App Auth.");
            let mut private_key = Vec::new();
            File::open(&app_auth.private_key_file)
                .expect("Opening the private key file")
                .read_to_end(&mut private_key)
                .expect("Reading the private key");

            return Github::new(
                String::from("NixOS/rfcs#39 (hubcaps)"),
                Credentials::InstallationToken(InstallationTokenGenerator::new(
                    app_auth.installation_id,
                    JWTCredentials::new(app_auth.app_id, private_key).unwrap(),
                )),
            )
            .expect("Failed to create a GitHub client from the app auth");
        }
        Err(e) => {
            app_auth_load_err = e;
        }
    }

    let token_auth_load_err: serde_json::error::Error;
    match nix::nix_instantiate_file_to_struct::<GitHubTokenAuth>(logger.new(o!()), credential_file)
    {
        Ok(token_auth) => {
            info!(
                logger,
                "Credential file is providing Token Auth, which cannot sync teams."
            );

            return Github::new(
                String::from("NixOS/rfcs#39 (hubcaps)"),
                Credentials::Token(token_auth.access_token),
            )
            .expect("Failed to create a GitHub client from the token auth");
        }
        Err(e) => {
            token_auth_load_err = e;
        }
    }

    error!(logger, "Credential file is not a valid App or Token auth method";
           "app_load" => ?app_auth_load_err,
           "token_load" => ?token_auth_load_err,
    );
    panic!("Credential file is not valid App or Token Auth");
}

fn execute_ops(logger: slog::Logger, inputs: Options) -> Result<(), ExitError> {
    // Note: I wanted these in a lazy_static!, but that meant metrics
    // which would report a 0 would never get reported at all, since
    // they aren't accessed.... and lazy_static! is lazy.
    let maintainer_nix_load_failure_counter = register_int_counter!(
        "rfc39_maintainer_nix_load_failure",
        "Failures to load maintainers.nix"
    )
    .unwrap();

    let maintainers = load_maintainer_file(logger.new(o!()), &inputs.maintainers)
        .map_err(|d| {
            maintainer_nix_load_failure_counter.inc();
            d
        })
        .unwrap();

    let github = gh_client_from_args(logger.new(o!()), &inputs.credential_file);

    match inputs.mode {
        ExecMode::CheckHandles => op_check_handles::check_handles(
            logger.new(o!("exec-mode" => "CheckHandles")),
            maintainers,
        ),
        ExecMode::BackfillIDs => op_backfill::backfill_ids(
            logger.new(o!("exec-mode" => "BackfillIDs")),
            github,
            &inputs.maintainers,
            maintainers,
        ),
        ExecMode::BlameAuthor => op_blame_author::report(
            logger.new(o!("exec-mode" => "BlameAuthor")),
            github,
            &inputs.maintainers,
            maintainers,
        ),
        ExecMode::SyncTeam(team_info) => op_sync_team::sync_team(
            logger.new(o!("exec-mode" => "SyncTeam")),
            github,
            maintainers,
            team_info.invited_list,
            &team_info.organization,
            team_info.team_id,
            team_info.dry_run,
            team_info.limit,
        ),
        ExecMode::ListTeams(team_info) => op_sync_team::list_teams(github, &team_info.organization),
    }
}

fn main() {
    let begin_counter =
        register_int_gauge!("rfc39_begin_seconds", "Execution started time").unwrap();
    begin_counter.set(
        time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap(),
    );

    let op_success_counter = register_int_counter!(
        "rfc39_op_suceess_counter",
        "Execution completed without fault."
    )
    .unwrap();
    let op_failed_counter =
        register_int_counter!("rfc39_op_failure_counter", "Execution failed").unwrap();
    let op_panic_counter = register_int_counter!(
        "rfc39_op_panic_counter",
        "Execution of the operation panicked"
    )
    .unwrap();

    let (logger, _scopes) = rfc39::default_logger();

    let mut inputs = Options::from_args();

    let dump_metrics = inputs.dump_metrics;
    let metrics_delay = inputs.metrics_delay;
    let metrics_handle = inputs.metrics_bind.take().map(|bind| {
        let bind = bind.parse().unwrap();
        let logger = logger.new(o!("thread" => "metrics"));
        thread::spawn(move || {
            info!(logger, "Listening on {:?}", bind);

            metrics::serve(&bind)
        })
    });

    let op_handle = {
        let logger = logger.new(o!());
        thread::spawn(move || {
            execute_ops(logger, inputs).map(|ok| {
                op_success_counter.inc();
                ok
            })
        })
    };

    let thread_result: Result<Result<(), ExitError>, _> = op_handle.join().map_err(|thread_err| {
        warn!(logger, "Op-handling child panicked: {:#?}", thread_err);
        op_failed_counter.inc();
        op_panic_counter.inc();
        thread_err
    });

    let exit_counter = register_int_gauge!("rfc39_stop_seconds", "Execution stopped time").unwrap();
    exit_counter.set(
        time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap(),
    );

    if let Some(_metrics_handle) = metrics_handle {
        thread::sleep(time::Duration::from_millis(1000 * metrics_delay));
        // we just let the metrics thread die at the end.
        // Never joined.
    }

    if dump_metrics {
        let mut buffer = Vec::<u8>::new();
        prometheus::default_registry();
        prometheus::TextEncoder::new()
            .encode(&prometheus::default_registry().gather(), &mut buffer)
            .unwrap();
        println!("metrics:\n {}", String::from_utf8(buffer).unwrap());
    }

    thread_result.unwrap().unwrap();
}
