{
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs";
    geng.url = "github:geng-engine/geng";
  };
  outputs = { self, nixpkgs, geng }: geng.makeFlakeOutputs
    (system:
      let pkgs = import nixpkgs { inherit system; };
      in
      {
        src = ./.;
        buildInputs = [ pkgs.openssl ];
      }) // {
    web =
      let
        system = "x86_64-linux";
        lib = geng.lib.${system};
        pkgs = import nixpkgs { inherit system; };
        buildGengPackage = import ./cargo-geng.nix { crane = lib.crane; cargo-geng = lib.cargo-geng; };
      in
      buildGengPackage {
        name = "ttv-web";
        src = ./.;
        target = "web";
      };
  };
}
