use crate::maintainers::{GitHubID, GitHubName, Handle};
use crate::nix;
use hubcaps::Github;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tokio::runtime::Runtime;

pub struct MaintainerHistory {
    logger: slog::Logger,
    barriers: Vec<String>,
    sources: Vec<(Vec<String>, HashMap<Handle, usize>)>,
}

impl MaintainerHistory {
    pub fn load(logger: slog::Logger, maintainer_file: &Path) -> MaintainerHistory {
        MaintainerHistory {
            logger: logger.clone(),
            barriers: vec![
                // sort and format
                "220459858b342ec880d484160eb63319b7b83af8".into(),
                // Convert maintainer file entries to attributes
                "f7da7fa0c3ab40b79a2358861831b925d2cb5a6b".into(),
                // alphabetize
                "dea3279593753f0dee2966cd3f0f1f84be5bfbe2".into(),
                // sort
                "a3a40b70892774792924824a9b8858a2ffd3489d".into(),
                // alphabetize
                "b4f60add6a227bfeb106497c270b8126dad8f8d3".into(),
                // insert-sort
                "a58a44e0c2106a87d258706f13cacc320adc8d32".into(),
                // alphabetize
                "ac1c3c95e18f6e9839f2ca151c761d1b283831f1".into(),
            ],
            sources: vec![
                // Record a list of breaks in the history of the maintainer
                // list. Capture the `.blame` file with `git blame -lb`
                // and capture the .nix file by just copying it out.
                //
                // Make sure to keep the list sorted by time.
                (
                    // current version from Git
                    git_blame_list(logger.clone(), maintainer_file).unwrap(),
                    maintainer_pos(logger.clone(), maintainer_file).unwrap(),
                ),
                load_old_data(
                    logger.clone(),
                    include_str!(
                        "../data/maintainer-list-05d273a45ed741d61ac6918361658c0c57b0ba41.blame"
                    ),
                    include_str!(
                        "../data/maintainer-list-05d273a45ed741d61ac6918361658c0c57b0ba41.nix"
                    ),
                ),
                // f7da7fa0c3ab40b79a2358861831b925d2cb5a6b...aa47bac04f06aeea993dc2e2cc6649fde4f31ed7
                // are all reverts around the maintainer list, so skipping those.
                // the next commit in the history is cf1b51aba2780fda582a18b1f97b1919339ddcd9,
                // so I checked that commit out and copied out the maintainer list &
                // `git blame -lb`'d the maintainer list.
                // Commit from: Sun Mar 4 00:46:25 2018 +0000
                load_old_data(
                    logger.clone(),
                    include_str!(
                        "../data/maintainer-list-cf1b51aba2780fda582a18b1f97b1919339ddcd9.blame"
                    ),
                    include_str!(
                        "../data/maintainer-list-cf1b51aba2780fda582a18b1f97b1919339ddcd9.nix"
                    ),
                ),
                // right after dea3279593753f0dee2966cd3f0f1f84be5bfbe2
                load_old_data(
                    logger.clone(),
                    include_str!(
                        "../data/maintainer-list-26b59efa8a747e82077e8430aa671db365d49b97.blame"
                    ),
                    include_str!(
                        "../data/maintainer-list-26b59efa8a747e82077e8430aa671db365d49b97.nix"
                    ),
                ),
                // right after a3a40b70892774792924824a9b8858a2ffd3489d
                load_old_data(
                    logger.clone(),
                    include_str!(
                        "../data/maintainer-list-822f480922fe2a0a38bc9de429cb2457b2eda96f.blame"
                    ),
                    include_str!(
                        "../data/maintainer-list-822f480922fe2a0a38bc9de429cb2457b2eda96f.nix"
                    ),
                ),
                // right after b4f60add6a227bfeb106497c270b8126dad8f8d3
                load_old_data(
                    logger.clone(),
                    include_str!(
                        "../data/maintainer-list-8e462995ba6deaeec9fd6dc6d3b9a110c08e5955.blame"
                    ),
                    include_str!(
                        "../data/maintainer-list-8e462995ba6deaeec9fd6dc6d3b9a110c08e5955.nix"
                    ),
                ),
                // right after a58a44e0c2106a87d258706f13cacc320adc8d32
                load_old_data(
                    logger.clone(),
                    include_str!(
                        "../data/maintainer-list-15c4a36012e6de9b335eb5576697279ad1cbbd48.blame"
                    ),
                    include_str!(
                        "../data/maintainer-list-15c4a36012e6de9b335eb5576697279ad1cbbd48.nix"
                    ),
                ),
                // right after ac1c3c95e18f6e9839f2ca151c761d1b283831f1
                load_old_data(
                    logger.clone(),
                    include_str!(
                        "../data/maintainer-list-9ce5fb002a7cf2369cddec8c25519ff73e0cf394.blame"
                    ),
                    include_str!(
                        "../data/maintainer-list-9ce5fb002a7cf2369cddec8c25519ff73e0cf394.nix"
                    ),
                ),
                /*
                load_old_data(
                    // Sort maintainer list
                    logger.clone(),
                    include_str!("../data/maintainer-list-d706fc953d0afe6bd060459f23f5e41a83c63a59.blame"),
                    include_str!("../data/maintainer-list-d706fc953d0afe6bd060459f23f5e41a83c63a59.nix"),
                    // Mon Sep 25 14:50:31 2017 +0100
                    "d706fc953d0afe6bd060459f23f5e41a83c63a59",
                ),
                */
            ],
        }
    }

    pub fn commit_for_user(&self, user: &Handle) -> Option<&str> {
        for (hash_list, positions) in &self.sources {
            trace!(self.logger, "Examining source for user";
                   "user" => %user,
            );

            if let Some(file_line) = positions.get(user) {
                if let Some(current_commit_hash) = hash_list.get(*file_line) {
                    if !self.barriers.contains(current_commit_hash) {
                        debug!(self.logger, "Identified source for user";
                               "user" => %user,
                               "file_line" => %file_line,
                               "current_commit_hash" => %current_commit_hash
                        );

                        return Some(current_commit_hash);
                    }
                }
            }
        }

        error!(self.logger, "Did not find a suitable commit for user";
               "user" => %user
        );

        None
    }

    pub fn confidence_for_user(
        &self,
        github: &Github,
        user: &Handle,
        github_name: &GitHubName,
        github_id: &GitHubID,
    ) -> Option<Confidence> {
        if let Some(hash) = self.commit_for_user(&user) {
            check_user_hash(&self.logger, &github, &user, &github_name, &github_id, hash)
        } else {
            warn!(self.logger, "Did not find a suitable commit hash for user";
                  "user" => %user,
            );
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Confidence {
    Total,
    BadAttribution,
    ChangedHandle,
    MismatchedNameAndID,
    CommitMissing,
}

fn check_user_hash(
    logger: &slog::Logger,
    github: &Github,
    user: &Handle,
    github_name: &GitHubName,
    github_id: &GitHubID,
    commit_hash: &str,
) -> Option<Confidence> {
    let mut rt = Runtime::new().unwrap();
    trace!(logger, "Looking up commit";
           "commit" => %commit_hash,
    );

    let commit = rt.block_on(github.repo("NixOS", "nixpkgs").commits().get(commit_hash));
    match commit {
        Ok(commit) => match (
            (GitHubName::new(commit.author.login.clone()) == *github_name),
            (GitHubID::new(commit.author.id) == *github_id),
            format!("{}", github_name).as_str(),
            commit.author.login.as_str(),
            commit_hash,
        ) {
            (true, true, _, _, _) => {
                debug!(logger, "Commit Details Match 100%";
                       "user" => %commit.author.login,
                       "commit" => %commit_hash,
                );

                Some(Confidence::Total)
            }

            // mismatch, mismatch, added user, added by, in commit
            (
                false,
                false,
                "rlupton20",
                "offlinehacker",
                "5bd136acd4c683b30470b5dfbb6f0b15dcea42a5",
            ) => Some(Confidence::Total),
            (false, false, "zx2c4", "Mic92", "6b1087d9b135c94b929fec3d4cf3724b9539c6b5") => {
                Some(Confidence::Total)
            }
            (false, false, "the-kenny", "bjornfor", "6b1087d9b135c94b929fec3d4cf3724b9539c6b5") => {
                Some(Confidence::Total)
            }

            (true, false, _, _, _) => {
                error!(logger, "Bug or recorded GitHub ID is wrong, as the ID does not match who authored the maintainer addition PR!";
                       "recorded_github_name" => %github_name,
                       "recorded_user_id" => %github_id,
                       "actual_user_id" => %commit.author.id,
                       "actual_github_name" => %commit.author.login,
                       "commit" => %commit_hash,
                );
                Some(Confidence::BadAttribution)
            }

            (false, true, _, _, _) => {
                warn!(logger, "Our user named {} changed their GitHub handle", user;
                      "recorded_github_name" => %github_name,
                      "actual_github_name" => %commit.author.login,
                      "github_user_id" => %github_id,
                      "commit" => %commit_hash,
                );
                Some(Confidence::ChangedHandle)
            }

            (false, false, _, _, _) => {
                warn!(logger, "Bug or recorded GitHub ID and GitHub Name is wrong, as neither the ID or the GitHub name match the author of the maintainer addition PR!";
                      "recorded_github_name" => %github_name,
                      "recorded_user_id" => %github_id,
                      "actual_user_id" => %commit.author.id,
                      "actual_github_name" => %commit.author.login,
                      "commit" => %commit_hash,
                );
                Some(Confidence::MismatchedNameAndID)
            }
        },
        Err(e) => {
            warn!(logger, "Failed to fetch commit";
                  "e" => %e,
                  "commit" => %commit_hash,
                  "handle" => %user,
            );
            Some(Confidence::CommitMissing)
        }
    }
}

fn git_blame_list(logger: slog::Logger, file: &Path) -> Result<Vec<String>, ()> {
    let output = Command::new("git")
        .args(&[
            "blame", "-l", // long commit hashes
            "-b", // show blank sha1s for boundary commits
        ])
        .arg(file)
        .current_dir(
            file.parent()
                .expect("Path to git blame has no parent, which is clearly a bug"),
        )
        .output()
        .expect("Failed to start git blame!");

    if !output.stderr.is_empty() {
        warn!(logger, "Stderr from git blame";
              "stderr" => String::from_utf8_lossy(&output.stderr).to_string()
        );
    }

    Ok(output
        .stdout
        .lines()
        .map(|line| line.expect("git blame output is unclean!"))
        .map(|line| {
            line.split(' ')
                .next()
                .expect("not even one space-separated element in git blame output!")
                .to_owned()
        })
        .collect())
}

fn load_old_data<'a>(
    logger: slog::Logger,
    blame: &str,
    nix: &str,
) -> (Vec<String>, HashMap<Handle, usize>) {
    let hash_list: Vec<String> = blame
        .lines()
        .map(|line| {
            line.split(' ')
                .next()
                .expect("not even one space-separated element in git blame output!")
                .to_owned()
        })
        .collect();

    let positions = {
        let tmpdir = tempfile::tempdir().unwrap();
        let file_path = tmpdir.path().join("old-maintainers.nix");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(nix.as_bytes()).unwrap();
        file.sync_all().unwrap();
        drop(file);

        let ret = maintainer_pos(logger.clone(), &file_path).unwrap();
        drop(tmpdir);
        ret
    };

    (hash_list, positions)
}

fn maintainer_pos(
    logger: slog::Logger,
    maintainer_file: &Path,
) -> Result<HashMap<Handle, usize>, serde_json::error::Error> {
    Ok(
        nix::nix_instantiate_expr_args_to_struct::<HashMap<Handle, usize>>(
            logger,
            r#"
{ maintainerFile }:
let
  maintainers = import maintainerFile;
  handles = builtins.attrNames maintainers;
in builtins.listToAttrs
(builtins.map
  (handle: {
    name = handle;
    value = (builtins.unsafeGetAttrPos handle maintainers).line;
   })
  handles)
"#,
            vec![("maintainerFile", maintainer_file.as_os_str())],
        )?
        .into_iter()
        .map(|(handle, size)| (handle, size - 1)) // Nix lines start at 1
        .collect(),
    )
}
