{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustPackages.cargo
    rustPackages.rustc
    git
    openssl
    pkg-config
    crate2nix
  ];

  RUST_BACKTRACE = "1";
  NIX_PATH = "nixpkgs=${pkgs.path}";
}
