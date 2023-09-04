{ crane, cargo-geng }:

{ target ? null
, ...
}@origArgs:
let
  cleanedArgs = builtins.removeAttrs origArgs [
    "installPhase"
    "installPhaseCommand"
    "target"
  ];

  crateName = crane.crateNameFromCargoToml cleanedArgs;

  # Avoid recomputing values when passing args down
  args = cleanedArgs // {
    pname = cleanedArgs.pname or crateName.pname;
    version = cleanedArgs.version or crateName.version;
    cargoVendorDir = cleanedArgs.cargoVendorDir or (crane.vendorCargoDeps cleanedArgs);
  };
in
crane.mkCargoDerivation (args // {
  # pnameSuffix = "-trunk";
  cargoArtifacts = args.cargoArtifacts or (crane.buildDepsOnly (args // {
    CARGO_BUILD_TARGET = args.CARGO_BUILD_TARGET or (if target == "web" then "wasm32-unknown-unknown" else target);
    installCargoArtifactsMode = args.installCargoArtifactsMode or "use-zstd";
    doCheck = args.doCheck or false;
  }));

  buildPhaseCargoCommand = args.buildPhaseCommand or ''
    local args=""
    if [[ "$CARGO_PROFILE" == "release" ]]; then
      args="$args --release"
    fi

    if [[ "${target}" == "web" ]]; then
      args="$args --web"
    fi

    cargo geng build $args
  '';

  installPhaseCommand = args.installPhaseCommand or ''
    cp -r target/geng $out
  '';

  # Installing artifacts on a distributable dir does not make much sense
  doInstallCargoArtifacts = args.doInstallCargoArtifacts or false;

  nativeBuildInputs = (args.nativeBuildInputs or [ ]) ++ [
    cargo-geng
  ];
})
