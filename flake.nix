{
  description = "srvcs-atan2: trigonometry: atan2(y, x)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        version = "0.1.0";
        rustToolchain = pkgs.rust-bin.stable."1.96.0".default.override {
          extensions = [ "clippy" "rustfmt" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        packages = {
          default = rustPlatform.buildRustPackage {
            pname = "srvcs-atan2";
            inherit version;
            src = ./.;
            cargoHash = "sha256-4ssFc7cvj3S54DE3KkO8e5HPpeA8r/O7fODXdGALa+0=";
          };
        } // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          container = pkgs.dockerTools.buildLayeredImage {
            name = "srvcs-atan2";
            tag = "latest";
            config = {
              Entrypoint = [ "${self.packages.${system}.default}/bin/srvcs-atan2" ];
              ExposedPorts = { "8080/tcp" = { }; };
              User = "65534:65534";
              Labels = {
                "org.opencontainers.image.title" = "srvcs-atan2";
                "org.opencontainers.image.description" = "srvcs-atan2: trigonometry: atan2(y, x).";
                "org.opencontainers.image.version" = version;
                "org.opencontainers.image.revision" = self.rev or "dev";
                "org.opencontainers.image.source" = "https://github.com/srvcs/atan2";
                "org.opencontainers.image.licenses" = "Apache-2.0";
              };
            };
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [ rustToolchain pkgs.syft ];
        };
      });
}
