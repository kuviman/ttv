{
  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs";
    geng.url = "github:geng-engine/geng";
  };
  outputs = { self, nixpkgs, geng }: geng.makeFlakeOutputs (system:
    let pkgs = import nixpkgs { inherit system; };
    in
    {
      src = ./.;
      buildInputs = [ pkgs.openssl ];
    });
}
