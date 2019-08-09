//! Given a checkout of nixpkgs, extract a dataset of GitHub account
//! information from the maintainer list.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct MaintainerList {
    maintainers: HashMap<Handle, Information>,
}

#[derive(Debug, PartialEq, Eq, Hash, Deserialize)]
pub struct Handle(String);
impl std::fmt::Display for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct GitHubName(String);
impl std::fmt::Display for GitHubName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct GitHubId(u128);
impl std::fmt::Display for GitHubId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Information {
    email: String,
    name: Option<String>,
    pub github: Option<GitHubName>,
    pub github_id: Option<GitHubId>,
}

impl MaintainerList {
    pub fn load(
        logger: slog::Logger,
        path: &Path,
    ) -> Result<MaintainerList, serde_json::error::Error> {
        Ok(MaintainerList {
            maintainers: nix_instantiate_to_struct(logger, path)?,
        })
    }
}

impl IntoIterator for MaintainerList {
    type Item = (Handle, Information);
    type IntoIter = std::collections::hash_map::IntoIter<Handle, Information>;

    fn into_iter(self) -> Self::IntoIter {
        self.maintainers.into_iter()
    }
}

fn nix_instantiate_to_struct<T>(
    logger: slog::Logger,
    file: &Path,
) -> Result<T, serde_json::error::Error>
where
    T: serde::de::DeserializeOwned,
{
    let output = Command::new("nix-instantiate")
        .args(&["--eval", "--strict", "--json"])
        .arg(file)
        .output()
        .expect("Failed to start nix-instantiate!");

    if !output.stderr.is_empty() {
        warn!(logger, "Stderr from nix-instantiate";
              "stderr" => String::from_utf8_lossy(&output.stderr).to_string()
        );
        // "stderr" => stderr);
    }

    serde_json::from_slice(&output.stdout)
}

#[cfg(test)]
mod tests {
    use super::{GitHubId, GitHubName, Handle, Information, MaintainerList};
    use std::path::Path;

    #[test]
    pub fn test_load_9175a201bbb28e679d72e9f7d28c84ab7d1f742_reduced() {
        let logger = rfc39::default_logger();

        let sample = Path::new("./samples/9175a201bbb28e679d72e9f7d28c84ab7d1f742b.reduced.nix");
        let expect = MaintainerList {
            maintainers: vec![
                (
                    Handle("0x4A6F".into()),
                    Information {
                        email: "0x4A6F@shackspace.de".into(),
                        name: Some("Joachim Ernst".into()),
                        github: Some(GitHubName("0x4A6F".into())),
                        github_id: None,
                    },
                ),
                (
                    Handle("1000101".into()),
                    Information {
                        email: "jan.hrnko@satoshilabs.com".into(),
                        name: Some("Jan Hrnko".into()),
                        github: Some(GitHubName("1000101".into())),
                        github_id: None,
                    },
                ),
                (
                    Handle("a1russell".into()),
                    Information {
                        email: "adamlr6+pub@gmail.com".into(),
                        name: Some("Adam Russell".into()),
                        github: None,
                        github_id: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(expect, MaintainerList::load(logger, &sample).unwrap(),);
    }

    #[test]
    pub fn test_load_9175a201bbb28e679d72e9f7d28c84ab7d1f742_proposed() {
        let logger = rfc39::default_logger();

        let sample = Path::new("./samples/9175a201bbb28e679d72e9f7d28c84ab7d1f742b.proposed.nix");
        let expect = MaintainerList {
            maintainers: vec![
                (
                    Handle("0x4A6F".into()),
                    Information {
                        email: "0x4A6F@shackspace.de".into(),
                        name: Some("Joachim Ernst".into()),
                        github: Some(GitHubName("0x4A6F".into())),
                        github_id: None,
                    },
                ),
                (
                    Handle("1000101".into()),
                    Information {
                        email: "jan.hrnko@satoshilabs.com".into(),
                        name: Some("Jan Hrnko".into()),
                        github: Some(GitHubName("1000101".into())),
                        github_id: Some(GitHubId(791309)),
                    },
                ),
                (
                    Handle("a1russell".into()),
                    Information {
                        email: "adamlr6+pub@gmail.com".into(),
                        name: Some("Adam Russell".into()),
                        github: None,
                        github_id: Some(GitHubId(241628)),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(expect, MaintainerList::load(logger, &sample).unwrap(),);
    }

    #[test]
    pub fn test_load_9175a201bbb28e679d72e9f7d28c84ab7d1f742() {
        let logger = rfc39::default_logger();

        let sample = Path::new("./samples/9175a201bbb28e679d72e9f7d28c84ab7d1f742b.nix");
        MaintainerList::load(logger, sample).unwrap();
    }

    #[test]
    pub fn test_load_stderr() {
        let logger = rfc39::default_logger();

        let sample = Path::new("./samples/stderr.nix");
        MaintainerList::load(logger, sample).unwrap();
    }
}
