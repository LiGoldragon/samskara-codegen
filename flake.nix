{
  description = "samskara-codegen — CozoDB schema → Cap'n Proto → Rust codegen";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    criome-cozo = { url = "github:LiGoldragon/criome-cozo"; flake = false; };
  };

  outputs = { self, nixpkgs, flake-utils, crane, fenix, criome-cozo, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rustToolchain = fenix.packages.${system}.latest.toolchain;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = craneLib.filterCargoSources;
        };

        commonArgs = {
          inherit src;
          pname = "samskara-codegen";
          postUnpack = ''
            depDir=$(dirname $sourceRoot)
            cp -rL ${criome-cozo} $depDir/criome-cozo
          '';
          nativeBuildInputs = [ pkgs.capnproto ];
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in
      {
        packages.default = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });

        checks = {
          build = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });
          tests = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [ rust-analyzer sqlite capnproto jujutsu ];
        };
      }
    );
}
