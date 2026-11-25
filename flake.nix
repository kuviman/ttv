{
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs";
    geng.url = "github:geng-engine/cargo-geng";
    geng.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs = inputs:
    let
      system = "x86_64-linux";
      geng = inputs.geng.lib;
      pkgs = import inputs.nixpkgs { inherit system; };
    in
    {
      packages.${system} = {
        default = geng.buildGengPackage {
          inherit system;
          src = ./.;
          buildInputs = [ pkgs.openssl ];
        };
        web = geng.buildGengPackage {
          inherit system;
          name = "ttv-web";
          src = ./.;
          platform = "web";
          cargoExtraArgs = "--target wasm32-unknown-unknown";
          buildInputs = [ pkgs.lld ];
        };
      };
      devShells.${system}.default = geng.mkShell {
        inherit system;
        target.linux.enable = true;
        target.web.enable = true;
      };
    };
}
