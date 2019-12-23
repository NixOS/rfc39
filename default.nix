{ pkgs ? import <nixpkgs> {} }:
(pkgs.callPackage ./Cargo.nix {}).rootCrate.build
