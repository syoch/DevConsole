{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.x86_64-linux.default = pkgs.rustPlatform.buildRustPackage {
        pname = "devconsole";
        version = "1.0.1";

        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      };
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rust-analyzer
          pkg-config
          udev
          rustfmt
          rustc
          clippy
          runit
          websocat

          git-conventional-commits
        ];
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        # SVDIR = (root)/.services
        shellHook = ''
          export RUST_BACKTRACE=1
          export SVDIR=$PWD/.services
        '';
      };
    };
}
