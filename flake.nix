{
  description = "Synchronizes the clipboard across multiple X11 and wayland instances running on the same machine";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };
  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = nixpkgs.legacyPackages;
      in {
	packages = forAllSystems (system: {
          default = pkgsFor.${system}.callPackage ./. { };
	});

	nixosModules.default =
	  # For illustration, probably want to break this definition out to a separate file
	  { config, pkgs, lib, ... }: {
            options = {
              services.clipboard-sync.enable = lib.mkEnableOption "clipboard-sync";
            };

            config = lib.mkIf config.services.clipboard-sync.enable {
              systemd.user.services.clipboard-sync = {
		description = "Synchronize clipboards across all displays";
		documentation = [ "https://github.com/dnut/clipboard-sync/" ];
		wantedBy = [ "graphical-session.target" ];
		after = [ "graphical-session.target" ];
		partOf = [ "graphical-session.target" ];
		requisite = [ "graphical-session.target" ];
		serviceConfig.ExecStart = "/usr/bin/env ${self.packages.${pkgs.system}.default}/bin/clipboard-sync --hide-timestamp --log-level debug";
		serviceConfig.Restart = "on-failure";
              };
            };
	  };
      };
}
