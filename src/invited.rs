use crate::cli::ExitError;
use crate::maintainers::GitHubID;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Invited {
    invited: HashSet<GitHubID>,
}

impl Invited {
    pub fn new() -> Invited {
        Invited {
            invited: HashSet::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Invited, ExitError> {
        // we want to create the file if it doesn't exist even though we won't
        // be writing to it, this just makes the API easier to use.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let lines = BufReader::new(file).lines();

        let mut invited = HashSet::new();
        for line in lines {
            invited.insert(GitHubID::new(line?.parse()?));
        }

        Ok(Invited { invited })
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

    pub fn contains(&self, id: &GitHubID) -> bool {
        self.invited.contains(id)
    }

    pub fn add(&mut self, id: GitHubID) {
        self.invited.insert(id);
    }

    pub fn remove(&mut self, id: &GitHubID) {
        self.invited.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_save() {
        let mut invited = Invited::new();
        let tmpdir = tempfile::tempdir().unwrap();
        let tmpfile = tmpdir.path().join("invited.txt");

        for n in (0..20).step_by(3) {
            invited.add(GitHubID::new(n));
        }

        invited.save(&tmpfile).unwrap();

        let loaded_invited = Invited::load(&tmpfile).unwrap();

        assert_eq!(invited, loaded_invited);
    }

    #[test]
    fn test_load_creates_file_if_doesnt_exist() {
        let tmpdir = tempfile::tempdir().unwrap();
        let tmpfile = tmpdir.path().join("invited.txt");

        let invited = Invited::load(&tmpfile).unwrap();

        assert_eq!(invited, Invited::new());
    }

    #[test]
    fn test_add_remove_invited() {
        let mut invited = Invited::new();

        assert!(!invited.contains(&GitHubID::new(0)));

        invited.add(GitHubID::new(0));
        assert!(invited.contains(&GitHubID::new(0)));

        invited.remove(&GitHubID::new(0));
        assert!(!invited.contains(&GitHubID::new(0)));
    }
}
