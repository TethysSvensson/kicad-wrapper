{
  rustPlatform,
  nix-gitignore,
}:

rustPlatform.buildRustPackage {
  pname = "kicad-wrapper";
  version = "0.1.0";
  src = nix-gitignore.gitignoreSource [ "*.nix" ] ./.;
  cargoHash = "sha256-7eA+Grdmuw23dGrHazx6+7PmI+qwOlID/3qYeYe2HMA=";
}
