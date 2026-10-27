{
  pkgs ? import <nixpkgs> { },
}:
pkgs.rustPlatform.buildRustPackage rec {
  buildInputs = with pkgs; [ libxcb ];

  pname = "clipboard-sync";
  version = "0.2.0";
  cargoLock.lockFile = ./Cargo.lock;
  src = ./.;
}
