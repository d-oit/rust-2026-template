{
  description = "A Nix-flake-based Rust development environment for rust-2026-template";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ inputs.self.overlays.default ];
            };
          }
        );
    in
    {
      overlays.default = final: prev: {
        # Mirrors rust-toolchain.toml: stable channel with clippy, rustfmt, rust-src.
        # Update the channel version here when rust-toolchain.toml is updated.
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine (
            with stable; [
              clippy
              rustc
              cargo
              rustfmt
              rust-src
            ]
          );
      };

      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              # Rust toolchain (matches rust-toolchain.toml)
              rustToolchain
              rust-analyzer

              # Build essentials
              openssl
              pkg-config
              clang
              mold

              # Cargo tools - mirrors CI pipeline jobs
              cargo-binstall   # fast binary installs
              cargo-deny       # supply chain / license checks (deny.toml)
              cargo-nextest    # test runner (.config/nextest.toml)
              cargo-audit      # security audit (CI security job)
              cargo-watch      # local file-watch rebuilds

              # Code quality
              typos            # typo linter (CI quality gate)
              just             # task runner (optional: add justfile)
              tokei            # line count utility

              # Uncomment for WASM targets:
              # wasm-bindgen-cli
              # rust-bindgen

              # Uncomment for web UI (Leptos) projects:
              # cargo-leptos
              # leptosfmt

              # Uncomment for desktop app (Tauri) projects:
              # cargo-tauri

              # Uncomment for database projects:
              # sqlx-cli
            ];

            env = {
              # Required by rust-analyzer and proc-macro expansion
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
          };
        }
      );
    };
}
