{
  description = "Implementation of NixOS RFC #39.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      overlays.default = final: prev: {
        rfc39 = (prev.callPackage ./Cargo.nix { }).rootCrate.build;
      };
      packages = (
        forAllSystems (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          in
          {
            rfc39 = pkgs.rfc39;
          }
        )
      );
    };
}
