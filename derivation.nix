{
  rustPlatform,
  nix-gitignore,
}:

rustPlatform.buildRustPackage {
  pname = "kicad-wrapper";
  version = "0.1.0";
  src = nix-gitignore.gitignoreSource [ "*.nix" ] ./.;
  cargoHash = "sha256-8RuMYVNTuzXVGlg02gbjUk+sBWJF4xW+5/Bu2GFaB8M=";
}
