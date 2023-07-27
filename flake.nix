{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, flake-compat, crane }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };

          lib = pkgs.lib;
          stdenv = pkgs.stdenv;
        in
        {
          packages.default =
            crane.lib.${system}.buildPackage
              {
                src = ./.;
              };

          devShell = pkgs.mkShell {
            buildInputs = [ pkgs.rustc pkgs.cargo pkgs.pre-commit ];

            shellHook = ''
              echo "Dev shell launched"
            '';
          };
        });

  nixConfig = {
    extra-substituters = [ "https://fiksn.cachix.org" ];
    extra-trusted-public-keys = [ "fiksn.cachix.org-1:BCEC7wp4PVp/atgIlbBSpNWOuPx7Zq4+cxwRqaMrSOc=" ];
  };
}
