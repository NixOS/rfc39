use crate::maintainers::{GitHubID, GitHubName, Handle, MaintainerList};
use futures::stream::Stream;
use hubcaps::teams::{Team, TeamMemberOptions, TeamMemberRole};
use hubcaps::users::User;
use hubcaps::Github;
use std::collections::HashMap;
use tokio::runtime::Runtime;

pub fn list_teams(github: Github, org: &str) {
    let mut rt = Runtime::new().unwrap();

    rt.block_on(github.org(org).teams().iter().for_each(|team| {
        println!("{:10} {}", team.id, team.name);
        Ok(())
    }))
    .expect("Failed to list teams");
}

pub fn sync_team(
    logger: slog::Logger,
    github: Github,
    maintainers: MaintainerList,
    org: &str,
    team_id: u64,
    dry_run: bool,
    limit: Option<u64>,
) {
    let mut rt = Runtime::new().unwrap();

    let team_actions = github.org(org).teams().get(team_id);
    let team = rt
        .block_on(team_actions.get())
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
        )
        .expect("Failed to fetch team members")
        .into_iter()
        .collect();

    let diff = maintainer_team_diff(maintainers, &current_members);
    let mut noops = 0;
    let mut additions = 0;
    let mut removals = 0;
    for (github_id, action) in diff {
        if let Some(limit) = limit {
            if (additions + removals) >= limit {
                info!(logger, "Hit maximum change limit";
                      "changed" => %(additions + removals),
                      "limit" => %format!("{:?}", limit),
                      "additions" => %additions,
                      "removals" => %removals,
                      "noops" => %noops,
                );
                return;
            }
        }
        match action {
            TeamAction::Add(github_name, handle) => {
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
                    );

                    // verify the ID and name still match
                    let user = rt
                        .block_on(github.users().get(&format!("{}", github_name)))
                        .unwrap();
                    if GitHubID::new(user.id) == github_id {
                        rt.block_on(team_actions.add_user(
                            &format!("{}", github_name),
                            TeamMemberOptions {
                                role: TeamMemberRole::Member,
                            },
                        ))
                        .unwrap();
                    } else {
                        warn!(logger, "Recorded username mismatch, not adding";
                              "nixpkgs-handle" => %handle,
                              "github-id" => %github_id,
                        );
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
                        email: "bob@example.com".into(),
                        name: Some("Bob".into()),
                        github: Some(GitHubName::new("bob")),
                        github_id: Some(GitHubID::new(2)),
                    },
                ),
                (
                    Handle::new("charlie"),
                    Information {
                        email: "charlie@example.com".into(),
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
