{ pkgs ? import <nixpkgs> { } }:
pkgs.rustPlatform.buildRustPackage rec {
  buildInputs = with pkgs; [
    xorg.libxcb
  ];

  pname = "clipboard-sync";
  version = "0.2.0";
  cargoLock.lockFile = ./Cargo.lock;
  src = ./.;
}
