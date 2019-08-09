//! Find a line like this:
//!
//!     github = "1000101";
//!
//! and see if 1000101 is in the list of IDs we have, and if so, there is
//! no githubId for that record... so,
//! inject in to the file:
//!
//!     githubId = THE_ID;
//!
//! Note, regex capture the leading whitespace from the `github =` line
//! to match indentation, no matter how janky it is.
//!
//! Then delete the ID from the hashmap.
//!
//! This might work:
//!
//!     ^(?<leading_space>\s+)github = "(?<name>[^"]*)";$
//!

use crate::maintainers::{GitHubID, GitHubName};
use regex::Regex;
use std::collections::HashMap;

pub fn backfill_file(mut ids: HashMap<GitHubName, GitHubID>, file: String) -> String {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"^(?P<leading_space>\s+)github = "(?P<name>[^"]*)";$"#).unwrap();
    }

    file.lines()
        .map(|line| {
            if let Some(matches) = RE.captures(line) {
                let username = matches
                    .name("name")
                    .expect("name should be in regex")
                    .as_str();

                if let Some(id) = ids.remove(&GitHubName::new(username.to_string())) {
                    let leading_space = matches
                        .name("leading_space")
                        .expect("leading_space should be in regex")
                        .as_str();

                    return format!("{}\n{}githubId = {};\n", line, leading_space, id);
                }
            }

            return format!("{}\n", line);
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::backfill_file;
    use crate::maintainers::{GitHubID, GitHubName};
    use std::fs::read_to_string;

    #[test]
    fn test_backfill_9175a201bbb28e679d72e9f7d28c84ab7d1f742b_reduced() {
        let input =
            read_to_string("./samples/9175a201bbb28e679d72e9f7d28c84ab7d1f742b.reduced.nix")
                .unwrap();

        let expect =
            read_to_string("./samples/9175a201bbb28e679d72e9f7d28c84ab7d1f742b.backfilled.nix")
                .unwrap();

        let output = backfill_file(
            vec![
                (GitHubName::new("1000101".into()), GitHubID::new(791309)),
                (GitHubName::new("0x4A6F".into()), GitHubID::new(9675338)),
            ]
            .into_iter()
            .collect(),
            input,
        );

        assert_eq!(expect, output);
    }
}
