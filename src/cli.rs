use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Options {
    /// Dump metrics to stdout after completion
    #[structopt(long = "dump-metrics")]
    pub dump_metrics: bool,

    /// Address and port information for binding the metrics server
    #[structopt(long = "metrics-addr")]
    pub metrics_bind: Option<String>,

    /// How long in seconds to keep the server up after operation is
    /// completed. Recommended to be 4x the scrape frequency.
    /// Only takes effect if metrics-addr is specified.
    /// Default: 240 seconds.
    #[structopt(long = "metrics-delay", default_value = "240")]
    pub metrics_delay: u64,

    /// Maintainer list
    #[structopt(short = "m", long = "maintainers", parse(from_os_str))]
    pub maintainers: PathBuf,

    /// GitHub Credential File
    #[structopt(short = "c", long = "credentials", parse(from_os_str))]
    pub credential_file: PathBuf,

    /// Execution Mode
    #[structopt(subcommand)]
    pub mode: ExecMode,
}

#[derive(Debug, StructOpt)]
pub enum ExecMode {
    /// Verify maintainers, their GitHub handle, and GitHub ID
    #[structopt(name = "check-handles")]
    CheckHandles,

    /// Poorly edit the maintainers.nix file to add missing GitHub IDs
    #[structopt(name = "backfill-ids")]
    BackfillIDs,

    /// Look to see if any of the GitHub handles have probably changed
    /// by examining who authored the commit adding the maintainer
    /// to the .nix file.
    #[structopt(name = "blame-author")]
    BlameAuthor,

    /// Add and remove team members from a GitHub team based on
    /// maintainership information. Use list-teams to find a team's
    /// ID
    #[structopt(name = "sync-team")]
    SyncTeam(SyncTeamParams),

    /// List an org's teams, to get the ID for sync-team
    #[structopt(name = "list-teams")]
    ListTeams(ListTeamParams),
}

#[derive(Debug, StructOpt)]
pub struct SyncTeamParams {
    pub organization: String,

    /// Find the team ID by going to
    pub team_id: u64,

    #[structopt(long = "dry-run")]
    pub dry_run: bool,

    #[structopt(long = "limit")]
    pub limit: Option<u64>,

    /// File to track previously invited users. Setting this parameter
    /// guarantees that users that have been previously invited and rejected
    /// will not keep getting spammed.
    #[structopt(long = "invited-list", parse(from_os_str))]
    pub invited_list: Option<PathBuf>,
}

#[derive(Debug, StructOpt)]
pub struct ListTeamParams {
    pub organization: String,
}

#[derive(Debug)]
pub enum ExitError {
    Io(std::io::Error),
    InvalidGitHubID(std::num::ParseIntError),
    Serde(serde_json::error::Error),
}

impl From<std::io::Error> for ExitError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<std::num::ParseIntError> for ExitError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::InvalidGitHubID(e)
    }
}

impl From<serde_json::error::Error> for ExitError {
    fn from(e: serde_json::error::Error) -> Self {
        Self::Serde(e)
    }
}
