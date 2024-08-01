{
  rustNightlyPlatform,
  lib,
}: let
  inherit ((builtins.fromTOML (builtins.readFile ./Cargo.toml)).package) version;
in
  rustNightlyPlatform.buildRustPackage {
    name = "schemat";
    inherit version;

    src = ./.;

    cargoHash = "sha256-t3mReU6J6s6HC3eGoCeAak0DW1n6e5c6/wKpw/w5nNg=";

    meta = with lib; {
      description = "Code formatter for Scheme, Lisp, and any S-expressions";
      repository = "https://github.com/raviqqe/schemat";
      license = licenses.unlicense;
      maintainers = with maintainers; [bddvlpr];
      mainProgram = "schemat";
    };
  }
