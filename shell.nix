{ pkgs ? import <nixpkgs> {
  overlays = [
    (import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz))
    (self: super: {
      crate2nix = self.callPackage
        (builtins.fetchTarball https://github.com/kolloch/crate2nix/tarball/master)
        {};
    })
  ];
}
}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    latest.rustChannels.stable.rust
    git
    openssl
    pkgconfig
    crate2nix
  ];

  RUST_BACKTRACE = "1";
  NIX_PATH = "nixpkgs=${pkgs.path}";
}
