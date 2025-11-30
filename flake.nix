{
  description = "Flake for installing/running/testing r8169_firmware";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, ... } @ inputs: flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import nixpkgs {inherit system;};
  in {
    devShell = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        clang
        rustup
        rust-bindgen
        gnumake
        bc
        bison
        flex
        elfutils
        ncurses
        linuxHeaders
        openssl
      ];
      packages = with pkgs; [wget lazygit ripgrep neovim git];
      shellHook = ''
        export FLAKE_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        export SCRIPTS_DIR="$FLAKE_ROOT/scripts"
        export KDIR="$FLAKE_ROOT/linux"
        export TMP="$FLAKE_ROOT/.tmp"
        source $SCRIPTS_DIR/utils.sh
        ensure_linux
      '';
    };
  }
  );
}
