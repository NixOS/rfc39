//! Given a checkout of nixpkgs, extract a dataset of GitHub account
//! information from the maintainer list.

use crate::nix;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

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
impl Handle {
    pub fn new<T>(name: T) -> Handle
    where
        T: Into<String>,
    {
        Handle(name.into())
    }
}

#[derive(Debug, Eq, Hash, Clone, Deserialize)]
pub struct GitHubName(String);
impl std::fmt::Display for GitHubName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl PartialEq for GitHubName {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}
impl GitHubName {
    pub fn new<T>(name: T) -> GitHubName
    where
        T: Into<String>,
    {
        GitHubName(name.into())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Deserialize)]
pub struct GitHubID(u64);
impl std::fmt::Display for GitHubID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl GitHubID {
    pub fn new(id: u64) -> GitHubID {
        GitHubID(id)
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Information {
    pub email: String,
    pub name: Option<String>,
    pub github: Option<GitHubName>,
    #[serde(rename = "githubId")]
    pub github_id: Option<GitHubID>,
}

impl MaintainerList {
    pub fn new(maintainers: HashMap<Handle, Information>) -> MaintainerList {
        MaintainerList { maintainers }
    }

    pub fn load(
        logger: slog::Logger,
        path: &Path,
    ) -> Result<MaintainerList, serde_json::error::Error> {
        Ok(MaintainerList {
            maintainers: nix::nix_instantiate_file_to_struct(logger, path)?,
        })
    }

    pub fn get<'a, 'b>(&'a self, handle: &'b Handle) -> Option<&'a Information> {
        self.maintainers.get(handle)
    }

    pub fn iter<'a>(&'a self) -> std::collections::hash_map::Iter<'a, Handle, Information> {
        self.maintainers.iter()
    }
}

impl IntoIterator for MaintainerList {
    type Item = (Handle, Information);
    type IntoIter = std::collections::hash_map::IntoIter<Handle, Information>;

    fn into_iter(self) -> Self::IntoIter {
        self.maintainers.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{GitHubID, GitHubName, Handle, Information, MaintainerList};
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
                        github_id: Some(GitHubID(791309)),
                    },
                ),
                (
                    Handle("a1russell".into()),
                    Information {
                        email: "adamlr6+pub@gmail.com".into(),
                        name: Some("Adam Russell".into()),
                        github: None,
                        github_id: Some(GitHubID(241628)),
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
