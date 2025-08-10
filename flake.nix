{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rust-analyzer
          pkg-config
          udev
          rustfmt
          rustc

          chromium
        ];
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        PUPPETEER_EXECUTABLE_PATH = "${pkgs.chromium}/bin/chromium";
        PUPPETEER_BROWSER = "chrome";
      };
    };
}
