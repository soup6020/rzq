{
  description = "rzq — command-line wrapper for rizin";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: import nixpkgs { inherit system; };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "rzq";
            version = "0.1.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkgs.makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/rzq \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.rizin ]}
            '';

            meta = {
              description = "Fast, colourful command-line wrapper for common Rizin analysis tasks";
              homepage = "https://github.com/soup6020/rzq";
              license = pkgs.lib.licenses.mit;
              mainProgram = "rzq";
              platforms = systems;
            };
          };
          rzq = self.packages.${system}.default;
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/rzq";
        };
      });

      checks = forAllSystems (system: {
        inherit (self.packages.${system}) default;
      });

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              rustc
              rustfmt
              rizin
            ];
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
    };
}
