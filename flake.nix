{
  description = "Linux driver for Attack Shark mice";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "attack-shark";
          version = "1.0.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          installPhase = ''
            runHook preInstall
            
            mkdir -p $out/bin
            cp target/release/attack-shark $out/bin/
            mkdir -p $out/etc/udev/rules.d
            cp ./99-attack-shark-r1.rules  $out/etc/udev/rules.d
            cp ./99-attack-shark-x11.rules $out/etc/udev/rules.d

            runHook postInstall
          '';
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
          ];
        };
      }
    );
}
