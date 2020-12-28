use crate::cli::ExitError;
use crate::maintainers::GitHubID;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[cfg_attr(test, derive(Debug))]
pub struct Invited {
    invited: HashSet<GitHubID>,
    logger: slog::Logger,
}

#[cfg(test)]
impl PartialEq for Invited {
    fn eq(&self, other: &Self) -> bool {
        self.invited == other.invited
    }
}

impl Invited {
    #[cfg(test)]
    pub fn new(logger: slog::Logger) -> Invited {
        Invited {
            invited: HashSet::new(),
            logger,
        }
    }

    pub fn load(logger: slog::Logger, path: &Path) -> Result<Invited, ExitError> {
        // we want to create the file if it doesn't exist even though we won't
        // be writing to it, this just makes the API easier to use.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(|err| {
                error!(
                    logger,
                    "Failed to open invited list file {:?}: {:?}", path, err
                );
                err
            })?;

        let lines = BufReader::new(file).lines();

        let mut invited = HashSet::new();
        for line in lines {
            let line = line.map_err(|err| {
                error!(
                    logger,
                    "Failed to read line from invited list file {:?}: {:?}", path, err
                );
                err
            })?;

            let id = line.parse().map_err(|err| {
                error!(
                    logger,
                    "Failed to parse invited maintainer github id: {:?}", err
                );
                err
            })?;

            invited.insert(GitHubID::new(id));
        }

        Ok(Invited { invited, logger })
    }

    pub fn save(&self, path: &Path) -> Result<(), ExitError> {
        let mut file = File::create(path).map_err(|err| {
            error!(
                self.logger,
                "Failed to create invited list file {:?}: {:?}", path, err,
            );
            err
        })?;

        let mut values = self.invited.iter().collect::<Vec<_>>();
        values.sort();

        let string = values
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        file.write_all(string.as_ref()).map_err(|err| {
            error!(
                self.logger,
                "Failed to write invited list file {:?}: {:?}", path, err
            );
            err
        })?;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.invited.len()
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
        let mut invited = Invited::new(rfc39::test_logger());
        let tmpdir = tempfile::tempdir().unwrap();
        let tmpfile = tmpdir.path().join("invited.txt");

        for n in (0..20).step_by(3) {
            invited.add(GitHubID::new(n));
        }

        invited.save(&tmpfile).unwrap();

        let loaded_invited = Invited::load(rfc39::test_logger(), &tmpfile).unwrap();

        assert_eq!(invited.len(), loaded_invited.len());

        assert_eq!(invited, loaded_invited);
    }

    #[test]
    fn test_load_creates_file_if_doesnt_exist() {
        let tmpdir = tempfile::tempdir().unwrap();
        let tmpfile = tmpdir.path().join("invited.txt");

        let invited = Invited::load(rfc39::test_logger(), &tmpfile).unwrap();

        assert_eq!(invited, Invited::new(rfc39::test_logger()));
    }

    #[test]
    fn test_add_remove_invited() {
        let mut invited = Invited::new(rfc39::test_logger());

        assert!(!invited.contains(&GitHubID::new(0)));

        invited.add(GitHubID::new(0));
        assert_eq!(invited.len(), 1);
        assert!(invited.contains(&GitHubID::new(0)));

        invited.remove(&GitHubID::new(0));
        assert_eq!(invited.len(), 0);
        assert!(!invited.contains(&GitHubID::new(0)));
    }
}
