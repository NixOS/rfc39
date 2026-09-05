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

  env = {
    RUST_BACKTRACE = "1";
    RUST_SRC_PATH = "${pkgs.rustPackages.rustPlatform.rustLibSrc}";
    NIX_PATH = "nixpkgs=${pkgs.path}";
  };
}
