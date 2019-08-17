use futures::stream::Stream;
use tokio::runtime::Runtime;
use hubcaps::Github;
use hubcaps::teams::Team;
use crate::maintainers::{MaintainerList};

pub fn list_teams(github: Github, org: &str, ) {
    let mut rt = Runtime::new().unwrap();

    rt.block_on(
        github
            .org(org)
            .teams()
            .iter()
            .for_each(|team| {
                println!("{:10} {}", team.id, team.name);
                Ok(())
            })
    ).expect("Failed to list teams");
}

pub fn sync_team(logger: slog::Logger, github: Github, maintainers: MaintainerList, org: &str, team_id: u64, ) {

    let mut rt = Runtime::new().unwrap();

    let team_actions = github.org(org).teams().get(team_id);
    let team = rt.block_on(team_actions.get()).expect("Failed to fetch team");
    info!(logger, "Syncing team";
          "team_name" => %team.name,
          "team_id" => %team.id,
    );
}
