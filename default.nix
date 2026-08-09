{
  pkgs ? import <nixpkgs> { },
}:

pkgs.rustPlatform.buildRustPackage {
  pname = "rzq";
  version = "0.1.0";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ pkgs.makeWrapper ];
  postInstall = ''
    wrapProgram $out/bin/rzq \
      --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.rizin ]}
  '';

  meta = {
    description = "Fast, colourful command-line wrapper for common Rizin analysis tasks";
    license = pkgs.lib.licenses.mit;
    mainProgram = "rzq";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
  };
}
