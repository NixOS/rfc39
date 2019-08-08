# From https://github.com/nixos/nixpkgs at:
#
#   Date: 2019-08-08 18:13:15
#  Commit: 9175a201bbb28e679d72e9f7d28c84ab7d1f742b
#
# Nixpkgs' license is MIT.
#
# Modified:
#  * deleted all but the first few, deleted the `github` handle from
#    the last entry.
#

/* List of NixOS maintainers.

    handle = {
      # Required
      name = "Your name";
      email = "address@example.org";

      # Optional
      github = "GithubUsername";
      keys = [{
        longkeyid = "rsa2048/0x0123456789ABCDEF";
        fingerprint = "AAAA BBBB CCCC DDDD EEEE  FFFF 0000 1111 2222 3333";
      }];
    };

  where

  - `handle` is the handle you are going to use in nixpkgs expressions,
  - `name` is your, preferably real, name,
  - `email` is your maintainer email address, and
  - `github` is your GitHub handle (as it appears in the URL of your profile page, `https://github.com/<userhandle>`),
  - `keys` is a list of your PGP/GPG key IDs and fingerprints.

  `handle == github` is strongly preferred whenever `github` is an acceptable attribute name and is short and convenient.

  Add PGP/GPG keys only if you actually use them to sign commits and/or mail.

  To get the required PGP/GPG values for a key run
  ```shell
  gpg --keyid-format 0xlong --fingerprint <email> | head -n 2
  ```

  !!! Note that PGP/GPG values stored here are for informational purposes only, don't use this file as a source of truth.

  More fields may be added in the future.

  Please keep the list alphabetically sorted.
  See `./scripts/check-maintainer-github-handles.sh` for an example on how to work with this data.
  */
{
  "0x4A6F" = {
    email = "0x4A6F@shackspace.de";
    name = "Joachim Ernst";
    github = "0x4A6F";
    keys = [{
      longkeyid = "rsa8192/0x87027528B006D66D";
      fingerprint = "F466 A548 AD3F C1F1 8C88  4576 8702 7528 B006 D66D";
    }];
  };
  "1000101" = {
    email = "jan.hrnko@satoshilabs.com";
    github = "1000101";
    name = "Jan Hrnko";
  };
  a1russell = {
    email = "adamlr6+pub@gmail.com";
    name = "Adam Russell";
  };
}
