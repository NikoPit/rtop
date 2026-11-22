{
  description = "A minimal process resource manager for the terminal";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "rxtop";
            version = "0.2.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            meta.description = "A minimal process resource manager for the terminal";
            meta.mainProgram = "rxtop";
          };
        });
    in {
      inherit packages;

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${packages.${system}.default}/bin/rxtop";
          meta.description = "A minimal process resource manager for the terminal";
        };
      });

      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in {
          default = pkgs.mkShell {
            packages = with pkgs; [ cargo rustc rustfmt clippy ];
          };
        });
    };
}
