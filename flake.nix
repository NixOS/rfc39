{
  description = "Implementation of NixOS RFC #39.";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, flake-utils }: {
    overlays.default = final: prev: {
      rfc39 = (prev.callPackage ./Cargo.nix {}).rootCrate.build;
    };
  } // (flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ self.overlays.default ];
      };
    in rec {
      packages.rfc39 = pkgs.rfc39;

      defaultPackage = packages.rfc39;
    }
  ));
}
