use crate::cli::ExitError;
use crate::invited::Invited;
use crate::maintainers::{GitHubID, GitHubName, Handle, MaintainerList};
use futures::stream::Stream;
use hubcaps::teams::{TeamMemberOptions, TeamMemberRole};
use hubcaps::Github;
use prometheus::{Histogram, IntCounter, IntGauge};
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::PathBuf;
use tokio::runtime::Runtime;

lazy_static! {
    static ref GITHUB_CALLS: IntCounter = register_int_counter!(
        "rfc39_github_call_count",
        "Code-level calls to GitHub API methods (not a count of actual calls made to GitHub.)"
    )
    .unwrap();
}

pub fn list_teams(github: Github, org: &str) -> Result<(), ExitError> {
    let mut rt = Runtime::new().unwrap();

    rt.block_on(github.org(org).teams().iter().for_each(|team| {
        println!("{:10} {}", team.id, team.name);
        Ok(())
    }))
    .expect("Failed to list teams");

    Ok(())
}

pub fn sync_team(
    logger: slog::Logger,
    github: Github,
    maintainers: MaintainerList,
    org: &str,
    team_id: u64,
    dry_run: bool,
    limit: Option<u64>,
    invited_list: Option<PathBuf>,
) -> Result<(), ExitError> {
    // initialize the counters :(
    GITHUB_CALLS.get();

    let get_team_histogram: Histogram =
        register_histogram!("rfc39_github_get_team", "Time to fetch a team").unwrap();
    let get_team_failures: IntCounter = register_int_counter!(
        "rfc39_github_get_team_failures",
        "Number of failed attempts to get a team"
    )
    .unwrap();

    let get_team_members_histogram: Histogram = register_histogram!(
        "rfc39_github_get_team_members",
        "Time to fetch team members"
    )
    .unwrap();
    let get_team_members_failures: IntCounter = register_int_counter!(
        "rfc39_github_get_team_members_failures",
        "Number of failed attempts to get a team's members"
    )
    .unwrap();
    let current_team_member_gauge: IntGauge =
        register_int_gauge!("rfc39_github_team_member_count", "Fetched team members").unwrap();

    let get_invitations_histogram: Histogram =
        register_histogram!("rfc39_github_get_invitations", "Time to fetch invitations").unwrap();
    let get_invitations_failures: IntCounter = register_int_counter!(
        "rfc39_github_get_team_invitation_failures",
        "Number of failed attempts to get a team's pending invitations"
    )
    .unwrap();
    let current_invitations_gauge: IntGauge =
        register_int_gauge!("rfc39_github_invitation_count", "Currently invited users").unwrap();

    let github_get_user_histogram: Histogram =
        register_histogram!("rfc39_github_get_user", "Time to fetch a GitHub user").unwrap();
    let github_get_user_failures: IntCounter = register_int_counter!(
        "rfc39_github_get_user_failures",
        "Number of failed attempts to get a user"
    )
    .unwrap();

    let github_add_user_histogram: Histogram = register_histogram!(
        "rfc39_github_add_user",
        "Time to add a GitHub user to a team"
    )
    .unwrap();
    let github_add_user_failures: IntCounter = register_int_counter!(
        "rfc39_github_add_user_failures",
        "Number of failed attempts to add a user"
    )
    .unwrap();

    let github_remove_user_histogram: Histogram = register_histogram!(
        "rfc39_github_remove_user",
        "Time to remove a GitHub user from a team"
    )
    .unwrap();
    let github_remove_user_failures: IntCounter = register_int_counter!(
        "rfc39_github_remove_user_failures",
        "Number of failed attempts to remove a user"
    )
    .unwrap();

    let github_user_unchanged_username_id_mismatch: IntGauge = register_int_gauge!(
        "rfc39_github_username_id_mismatch",
        "Number of maintainers not added because of out of date usernames, due to a mismatched ID"
    )
    .unwrap();

    let mut rt = TrackedReactor {
        rt: Runtime::new().unwrap(),
    };

    let do_it_live = !dry_run;

    let team_actions = github.org(org).teams().get(team_id);
    let team = rt
        .block_on(team_actions.get(), &get_team_histogram, &get_team_failures)
        .expect("Failed to fetch team");

    info!(logger, "Syncing team";
          "team_name" => %team.name,
          "team_id" => %team.id,
    );

    info!(logger, "Fetching current team members";
          "team_name" => %team.name,
          "team_id" => %team.id,
    );

    let current_members: HashMap<GitHubID, GitHubName> = rt
        .block_on(
            team_actions
                .iter_members()
                .map(|user| (GitHubID::new(user.id), GitHubName::new(user.login)))
                .collect(),
            &get_team_members_histogram,
            &get_team_members_failures,
        )
        .expect("Failed to fetch team members")
        .into_iter()
        .collect();

    current_team_member_gauge.set(current_members.len().try_into().unwrap());

    let mut invited = if let Some(ref invited_list) = invited_list {
        Invited::load(invited_list)?
    } else {
        Invited::new()
    };

    debug!(logger, "Fetching existing invitations");
    let pending_invites: Vec<GitHubName> = rt
        .block_on(
            github
                .org(org)
                .membership()
                .invitations()
                .filter_map(|invite| Some(GitHubName::new(invite.login?)))
                .collect(),
            &get_invitations_histogram,
            &get_invitations_failures,
        )
        .expect("failed to list existing invitations");
    current_invitations_gauge.set(pending_invites.len().try_into().unwrap());

    debug!(logger, "Fetched invitations.";
           "pending_invitations" => pending_invites.len()
    );

    let diff = maintainer_team_diff(maintainers, &current_members);

    let limit_metric = register_int_gauge!(
        "rfc39_team_sync_change_limit",
        "Total number of additions and changed allowed in a single run"
    )
    .unwrap();
    if let Some(limit) = limit {
        limit_metric.set(limit.try_into().unwrap());
    }

    let limit: Option<i64> = limit.map(|lim| lim.try_into().unwrap());

    let noops = register_int_counter!(
        "rfc39_team_sync_noops",
        "Total count of noop team sync actions"
    )
    .unwrap();
    let additions =
        register_int_counter!("rfc39_team_sync_additions", "Total team additions").unwrap();
    let removals =
        register_int_counter!("rfc39_team_sync_removals", "Total team removals").unwrap();
    let errors = register_int_counter!("rfc39_team_sync_errors", "Total team errors").unwrap();
    for (github_id, action) in diff {
        let logger = logger.new(o!(
            "dry-run" => dry_run,
            "github-id" => format!("{}", github_id),
            "changed" => additions.get() + removals.get(),
            "additions" => additions.get(),
            "removals" => removals.get(),
            "noops" => noops.get(),
            "errors" => errors.get(),
        ));
        if let Some(limit) = limit {
            if (additions.get() + removals.get()) >= limit {
                info!(logger, "Hit maximum change limit");
                return Ok(());
            }
        }
        match action {
            TeamAction::Add(github_name, github_id, handle) => {
                let logger = logger.new(o!(
                    "nixpkgs-handle" => format!("{}", handle),
                    "github-name" => format!("{}", github_name),
                ));

                if pending_invites.contains(&github_name) {
                    noops.inc();
                    debug!(logger, "User already has a pending invitation");
                } else if invited.contains(&github_id) {
                    noops.inc();
                    debug!(logger, "User was already invited previously (since there's no pending invitation we can assume the user rejected the invite)");
                } else {
                    additions.inc();
                    info!(logger, "Adding user to the team");

                    if do_it_live {
                        // verify the ID and name still match
                        let get_user = rt.block_on(
                            github.users().get(&format!("{}", github_name)),
                            &github_get_user_histogram,
                            &github_get_user_failures,
                        )
                            .map_err(|e| {
                                errors.inc();
                                warn!(logger, "Failed to fetch user by name, incrementing noops. error: {:#?}", e);
                                e
                            })
                            .map(|user| {
                                if GitHubID::new(user.id) != github_id {
                                    github_user_unchanged_username_id_mismatch.inc();
                                    warn!(logger, "Recorded username mismatch, not adding");
                                    None
                                } else {
                                    Some(user)
                                }
                            });

                        if let Ok(Some(_user)) = get_user {
                            let add_attempt = rt.block_on(
                                team_actions.add_user(
                                    &format!("{}", github_name),
                                    TeamMemberOptions {
                                        role: TeamMemberRole::Member,
                                    },
                                ),
                                &github_add_user_histogram,
                                &github_add_user_failures,
                            );

                            match add_attempt {
                                Ok(_) => {
                                    // keep track of the invitation locally so that we don't
                                    // spam users that have already been invited and rejected
                                    // the invitation
                                    invited.add(github_id.clone());
                                }
                                Err(e) => {
                                    errors.inc();
                                    warn!(logger, "Failed to add a user to the team, not decrementing additions as it may have succeeded: {:#?}", e);
                                }
                            }
                        }
                    }
                }
            }
            TeamAction::Keep(handle) => {
                let logger = logger.new(o!(
                    "nixpkgs-handle" => format!("{}", handle),
                ));

                noops.inc();
                trace!(logger, "Keeping user on the team");
            }
            TeamAction::Remove(github_name, github_id) => {
                let logger = logger.new(o!(
                    "github-name" => format!("{}", github_name),                ));

                removals.inc();
                info!(logger, "Removing user from the team");
                if do_it_live {
                    // verify the ID and name still match
                    let get_user = rt
                        .block_on(
                            github.users().get(&format!("{}", github_name)),
                            &github_get_user_histogram,
                            &github_get_user_failures,
                        )
                        .map_err(|e| {
                            errors.inc();
                            warn!(
                                logger,
                                "Failed to fetch user by name, incrementing noops. error: {:#?}", e
                            );
                            e
                        })
                        .map(|user| {
                            if GitHubID::new(user.id) != github_id {
                                github_user_unchanged_username_id_mismatch.inc();
                                warn!(logger, "Recorded username mismatch, not adding");
                                None
                            } else {
                                Some(user)
                            }
                        });

                    if let Ok(Some(_)) = get_user {
                        let remove_attempt = rt.block_on(
                            team_actions.remove_user(&format!("{}", github_name)),
                            &github_remove_user_histogram,
                            &github_remove_user_failures,
                        );

                        match remove_attempt {
                            Ok(_) => invited.remove(&github_id),
                            Err(e) => {
                                errors.inc();
                                warn!(logger, "Failed to remove a user from the team: {:#?}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(ref invited_list) = invited_list {
        invited.save(invited_list)?;
    }

    Ok(())
}

struct TrackedReactor {
    rt: Runtime,
}

impl TrackedReactor {
    fn block_on<F, E, I>(
        &mut self,
        what: F,
        histogram: &Histogram,
        fails: &IntCounter,
    ) -> Result<I, E>
    where
        F: Send + 'static + futures::future::Future<Item = I, Error = E>,
        E: Send + 'static,
        I: Send + 'static,
    {
        GITHUB_CALLS.inc();
        let _timer = histogram.start_timer();
        self.rt.block_on(what).map_err(|e| {
            fails.inc();
            e
        })
    }
}

#[derive(Debug, PartialEq)]
enum TeamAction {
    Add(GitHubName, GitHubID, Handle),
    Remove(GitHubName, GitHubID),
    Keep(Handle),
}

fn maintainer_team_diff(
    maintainers: MaintainerList,
    teammembers: &HashMap<GitHubID, GitHubName>,
) -> HashMap<GitHubID, TeamAction> {
    let missing_github_handle = register_int_gauge!(
        "rfc39_maintainer_missing_key_github",
        "Maintainers missing a github handle."
    )
    .unwrap();
    let missing_github_id = register_int_gauge!(
        "rfc39_maintainer_missing_key_github_id",
        "Maintainers missing a github_id."
    )
    .unwrap();

    let mut diff: HashMap<GitHubID, TeamAction> = maintainers
        .into_iter()
        .inspect(|(_, maintainer)| {
            if maintainer.github.is_none() {
                missing_github_handle.inc();
            }
            if maintainer.github_id.is_none() {
                missing_github_id.inc();
            }
        })
        .filter_map(|(handle, m)| {
            if teammembers.contains_key(&m.github_id?) {
                Some((m.github_id?, TeamAction::Keep(handle)))
            } else {
                Some((
                    m.github_id?,
                    TeamAction::Add(m.github?, m.github_id?, handle),
                ))
            }
        })
        .collect();

    for (github_id, github_name) in teammembers {
        // the diff list already has an entry for who should be in it
        // now create removals for who should no longer be present
        if !diff.contains_key(github_id) {
            diff.insert(
                *github_id,
                TeamAction::Remove(github_name.clone(), *github_id),
            );
        }
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maintainers::Information;

    #[test]
    fn test_add_remove_members() {
        let on_github: HashMap<GitHubID, GitHubName> = vec![
            (GitHubID::new(1), GitHubName::new("alice")),
            (GitHubID::new(2), GitHubName::new("bob")),
        ]
        .into_iter()
        .collect();

        let wanted = MaintainerList::new(
            vec![
                (
                    Handle::new("bob"),
                    Information {
                        email: Some("bob@example.com".into()),
                        name: Some("Bob".into()),
                        github: Some(GitHubName::new("bob")),
                        github_id: Some(GitHubID::new(2)),
                    },
                ),
                (
                    Handle::new("charlie"),
                    Information {
                        email: Some("charlie@example.com".into()),
                        name: Some("Charlie".into()),
                        github: Some(GitHubName::new("charlie")),
                        github_id: Some(GitHubID::new(3)),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            vec![
                (
                    GitHubID::new(1),
                    TeamAction::Remove(GitHubName::new("alice"), GitHubID::new(1))
                ),
                (GitHubID::new(2), TeamAction::Keep(Handle::new("bob"))),
                (
                    GitHubID::new(3),
                    TeamAction::Add(
                        GitHubName::new("charlie"),
                        GitHubID::new(3),
                        Handle::new("charlie")
                    )
                ),
            ]
            .into_iter()
            .collect::<HashMap<GitHubID, TeamAction>>(),
            maintainer_team_diff(wanted, &on_github)
        );
    }
}
