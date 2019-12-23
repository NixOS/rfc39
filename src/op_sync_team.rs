use crate::maintainers::{GitHubID, GitHubName, Handle, MaintainerList};
use futures::stream::Stream;
use hubcaps::teams::{TeamMemberOptions, TeamMemberRole};
use std::convert::TryInto;
use crate::cli::ExitError;
use hubcaps::Github;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use prometheus::{IntCounter, IntGauge, Histogram};

lazy_static! {
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
) -> Result<(), ExitError> {
    let get_team_histogram: Histogram = register_histogram!("github_get_team", "Time to fetch a team").unwrap();
    let get_team_failures: IntCounter = register_int_counter!("github_get_team_failures", "Number of failed attempts to get a team").unwrap();

    let get_team_members_histogram: Histogram = register_histogram!("github_get_team_members", "Time to fetch team members").unwrap();
    let get_team_members_failures: IntCounter = register_int_counter!("github_get_team_members_failures", "Number of failed attempts to get a team's members").unwrap();
    let current_team_member_gauge: IntGauge = register_int_gauge!("github_team_member_count", "Fetched team members").unwrap();

    let get_invitations_histogram: Histogram = register_histogram!("github_get_invitations", "Time to fetch invitations").unwrap();
    let get_invitations_failures: IntCounter = register_int_counter!("github_get_team_invitation_failures", "Number of failed attempts to get a team's pending invitations").unwrap();
    let current_invitations_gauge: IntGauge = register_int_gauge!("github_invitation_count", "Currently invited users").unwrap();

    let github_calls: IntCounter = register_int_counter!("github_call_count", "Code-level calls to GitHub API methods (not a count of actual calls made to GitHub.)").unwrap();


    let mut rt = Runtime::new().unwrap();

    let team_actions = github.org(org).teams().get(team_id);
    let team = {
        github_calls.inc();
        let _timer = get_team_histogram.start_timer();
        rt
            .block_on(team_actions.get())
            .map_err(|e| {
                get_team_failures.inc();
                e
            })
            .expect("Failed to fetch team")
    };
    info!(logger, "Syncing team";
          "team_name" => %team.name,
          "team_id" => %team.id,
    );

    info!(logger, "Fetching current team members";
          "team_name" => %team.name,
          "team_id" => %team.id,
    );

    let current_members: HashMap<GitHubID, GitHubName> = {
        github_calls.inc();
        let _timer = get_team_members_histogram.start_timer();
        rt
            .block_on(
                team_actions
                    .iter_members()
                    .map(|user| (GitHubID::new(user.id), GitHubName::new(user.login)))
                    .collect(),
            )
            .map_err(|e| {
                get_team_members_failures.inc();
                e
            })
            .expect("Failed to fetch team members")
            .into_iter()
            .collect()
    };
    current_team_member_gauge.set(current_members.len().try_into().unwrap());

    debug!(logger, "Fetching existing invitations");
    let pending_invites: Vec<GitHubName> = {
        github_calls.inc();
        let _timer = get_invitations_histogram.start_timer();
        rt
            .block_on(
                github
                    .org(org)
                    .membership()
                    .invitations()
                    .filter_map(|invite| Some(GitHubName::new(invite.login?)))
                    .collect(),
            )
            .map_err(|e| {
                get_invitations_failures.inc();
                e
            })
            .expect("failed to list existing invitations")
    };
    current_invitations_gauge.set(pending_invites.len().try_into().unwrap());

    debug!(logger, "Fetched invitations.";
           "pending_invitations" => pending_invites.len()
    );

    let diff = maintainer_team_diff(maintainers, &current_members);
    let mut noops = 0;
    let mut additions = 0;
    let mut removals = 0;
    let mut errors = 0;
    for (github_id, action) in diff {
        if let Some(limit) = limit {
            if (additions + removals) >= limit {
                info!(logger, "Hit maximum change limit";
                      "changed" => %(additions + removals),
                      "limit" => %format!("{:?}", limit),
                      "additions" => %additions,
                      "removals" => %removals,
                      "noops" => %noops,
                      "errors" => %errors,
                );
                return Ok(());
            }
        }
        match action {
            TeamAction::Add(github_name, handle) => {
                if pending_invites.contains(&github_name) {
                    noops += 1;
                    debug!(logger, "User already has a pending invitation";
                           "nixpkgs-handle" => %handle,
                           "github-name" => %github_name,
                           "github-id" => %github_id,
                           "changed" => %(additions + removals),
                           "limit" => %format!("{:?}", limit),
                           "additions" => %additions,
                           "removals" => %removals,
                           "noops" => %noops,
                           "errors" => %errors,
                    );
                } else {
                    additions += 1;
                    if dry_run {
                        info!(logger, "Would add user to the team";
                              "nixpkgs-handle" => %handle,
                              "github-name" => %github_name,
                              "github-id" => %github_id,
                              "changed" => %(additions + removals),
                              "limit" => %format!("{:?}", limit),
                              "additions" => %additions,
                              "removals" => %removals,
                              "noops" => %noops,
                              "errors" => %errors,
                        );
                    } else {
                        info!(logger, "Adding user to the team";
                              "nixpkgs-handle" => %handle,
                              "github-name" => %github_name,
                              "github-id" => %github_id,
                              "changed" => %(additions + removals),
                              "limit" => %format!("{:?}", limit),
                              "additions" => %additions,
                              "removals" => %removals,
                              "noops" => %noops,
                              "errors" => %errors,
                        );

                        // verify the ID and name still match
                        match rt.block_on(github.users().get(&format!("{}", github_name))) {
                            Ok(user) => {
                                if GitHubID::new(user.id) == github_id {
                                    match rt.block_on(team_actions.add_user(
                                        &format!("{}", github_name),
                                        TeamMemberOptions {
                                            role: TeamMemberRole::Member,
                                        },
                                    )) {
                                        Ok(_) => (),
                                        Err(e) => {
                                            errors += 1;
                                            warn!(logger, "Failed to add a user to the team, not decrementing additions as it may have succeeded: {:#?}", e;
                                                  "nixpkgs-handle" => %handle,
                                                  "github-name" => %github_name,
                                                  "github-id" => %github_id,
                                                  "changed" => %(additions + removals),
                                                  "limit" => %format!("{:?}", limit),
                                                  "additions" => %additions,
                                                  "removals" => %removals,
                                                  "noops" => %noops,
                                                  "errors" => %errors,
                                            );
                                        }
                                    }
                                } else {
                                    warn!(logger, "Recorded username mismatch, not adding";
                                          "nixpkgs-handle" => %handle,
                                          "github-id" => %github_id,
                                    );
                                }
                            }
                            Err(e) => {
                                additions -= 1;
                                errors += 1;
                                warn!(logger, "Failed to fetch user by name, decrementing additions, incrementing noops. error: {:#?}", e;
                                      "nixpkgs-handle" => %handle,
                                      "github-name" => %github_name,
                                      "github-id" => %github_id,
                                      "changed" => %(additions + removals),
                                      "limit" => %format!("{:?}", limit),
                                      "additions" => %additions,
                                      "removals" => %removals,
                                      "noops" => %noops,
                                      "errors" => %errors,
                                );
                            }
                        }
                    }
                }
            }
            TeamAction::Keep(handle) => {
                noops += 1;
                trace!(logger, "Keeping user on the team";
                       "nixpkgs-handle" => %handle,
                       "github-id" => %github_id,
                       "changed" => %(additions + removals),
                       "limit" => %format!("{:?}", limit),
                       "additions" => %additions,
                       "removals" => %removals,
                       "noops" => %noops,
                );
            }
            TeamAction::Remove(handle) => {
                removals += 1;
                if dry_run {
                    info!(logger, "Would remove user from the team";
                          "nixpkgs-handle" => %handle,
                          "github-id" => %github_id,
                          "changed" => %(additions + removals),
                          "limit" => %format!("{:?}", limit),
                          "additions" => %additions,
                          "removals" => %removals,
                          "noops" => %noops,
                    );
                } else {
                    info!(logger, "Removing user from the team";
                          "nixpkgs-handle" => %handle,
                          "github-id" => %github_id,
                          "changed" => %(additions + removals),
                          "limit" => %format!("{:?}", limit),
                          "additions" => %additions,
                          "removals" => %removals,
                          "noops" => %noops,
                    );
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
enum TeamAction {
    Add(GitHubName, Handle),
    Remove(GitHubName),
    Keep(Handle),
}

fn maintainer_team_diff(
    maintainers: MaintainerList,
    teammembers: &HashMap<GitHubID, GitHubName>,
) -> HashMap<GitHubID, TeamAction> {
    let mut diff: HashMap<GitHubID, TeamAction> = maintainers
        .into_iter()
        .filter_map(|(handle, m)| {
            if teammembers.contains_key(&m.github_id?) {
                Some((m.github_id?, TeamAction::Keep(handle)))
            } else {
                Some((m.github_id?, TeamAction::Add(m.github?, handle)))
            }
        })
        .collect();

    for (github_id, github_name) in teammembers {
        // the diff list already has an entry for who should be in it
        // now create removals for who should no longer be present
        if !diff.contains_key(github_id) {
            diff.insert(*github_id, TeamAction::Remove(github_name.clone()));
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
                    TeamAction::Remove(GitHubName::new("alice"))
                ),
                (GitHubID::new(2), TeamAction::Keep(Handle::new("bob"))),
                (
                    GitHubID::new(3),
                    TeamAction::Add(GitHubName::new("charlie"), Handle::new("charlie"))
                ),
            ]
            .into_iter()
            .collect::<HashMap<GitHubID, TeamAction>>(),
            maintainer_team_diff(wanted, &on_github)
        );
    }
}
