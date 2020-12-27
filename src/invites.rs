use crate::cli::ExitError;
use crate::maintainers::GitHubID;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Invites {
    invited: HashSet<GitHubID>,
}

impl Invites {
    pub fn new() -> Invites {
        Invites {
            invited: HashSet::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Invites, ExitError> {
        let file = File::open(path)?;
        let lines = BufReader::new(file).lines();

        let mut invited = HashSet::new();
        for line in lines {
            invited.insert(GitHubID::new(line?.parse()?));
        }

        Ok(Invites { invited })
    }

    pub fn save(&self, path: &Path) -> Result<(), ExitError> {
        let mut file = File::create(path)?;

        let string = self
            .invited
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        file.write_all(string.as_ref())?;

        Ok(())
    }

    pub fn invited(&self, id: &GitHubID) -> bool {
        self.invited.contains(id)
    }

    pub fn add_invite(&mut self, id: GitHubID) {
        self.invited.insert(id);
    }

    pub fn remove_invite(&mut self, id: &GitHubID) {
        self.invited.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_save() {
        let mut invites = Invites::new();
        let tmpdir = tempfile::tempdir().unwrap();
        let tmpfile = tmpdir.path().join("invites.txt");

        for n in (0..20).step_by(3) {
            invites.add_invite(GitHubID::new(n));
        }

        invites.save(&tmpfile).unwrap();

        let loaded_invites = Invites::load(&tmpfile).unwrap();

        assert_eq!(invites, loaded_invites);
    }

    #[test]
    fn test_add_remove_invites() {
        let mut invites = Invites::new();

        assert!(!invites.invited(&GitHubID::new(0)));

        invites.add_invite(GitHubID::new(0));
        assert!(invites.invited(&GitHubID::new(0)));

        invites.remove_invite(&GitHubID::new(0));
        assert!(!invites.invited(&GitHubID::new(0)));
    }
}
