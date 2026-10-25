{
  description = "CrabER: ER diagram tool";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
          ];
        };

        inherit (pkgs) lib;

        # The dev shell's toolchain carries rust-analyzer and rust-src, which a
        # package build has no use for. Same channel, so `nix build` cannot pass
        # on a compiler the dev shell would reject.
        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.stable.latest.minimal;
          rustc = pkgs.rust-bin.stable.latest.minimal;
        };

        # fetchPnpmDeps is a fixed-output derivation: changing its inputs re-runs
        # the fetch even when the resolved store would be identical. Feeding it
        # only the two files that decide what gets downloaded means editing a
        # component does not re-download node_modules.
        pnpmLockSrc = lib.fileset.toSource {
          root = ./web;
          fileset = lib.fileset.unions [
            ./web/package.json
            ./web/pnpm-lock.yaml
          ];
        };

        # Enumerated rather than filtered: lib.cleanSource keeps node_modules,
        # which is hundreds of megabytes of store for nothing.
        webSrc = lib.fileset.toSource {
          root = ./web;
          fileset = lib.fileset.unions [
            ./web/package.json
            ./web/pnpm-lock.yaml
            ./web/tsconfig.json
            ./web/vite.config.ts
            ./web/index.html
            ./web/src
            ./web/public
          ];
        };

        craber-web = pkgs.stdenv.mkDerivation (finalAttrs: {
          pname = "craber-web";
          version = "0.0.0";
          src = webSrc;

          # pnpm_11 rather than pkgs.pnpm: it is exactly the 11.27.0 that
          # web/package.json pins, and the store layout changes across majors.
          nativeBuildInputs = [
            pkgs.nodejs_26
            pkgs.pnpm_11
            pkgs.pnpmConfigHook
          ];

          pnpmDeps = pkgs.fetchPnpmDeps {
            inherit (finalAttrs) pname version src;
            pnpm = pkgs.pnpm_11;
            # Mandatory, and 4 specifically: 1 and 2 were removed in nixpkgs
            # 26.11 and 3 is rejected outright for pnpm_11 and newer.
            fetcherVersion = 4;
            hash = "sha256-VK6a2YW6e1AvBGINccEE50Tvesqif71amyjk1d0wpMk=";
          };

          buildPhase = ''
            runHook preBuild
            pnpm build
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            cp -R dist $out
            # Whatever schema the dev server last looked at is not part of the
            # bundle; `craber build` writes the real one. Matches the skip in
            # crates/craber-site/build.rs and `just vendor-bundle`.
            rm -f $out/schema.json
            runHook postInstall
          '';

          dontFixup = true;
        });

        craber = rustPlatform.buildRustPackage {
          pname = "craber";
          version = "0.0.0";

          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./README.md
              ./crates
              # The install check renders one, and it is the only way to prove
              # the bundle really made it in.
              ./fixtures
            ];
          };

          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "-p"
            "craber"
          ];

          # The first candidate crates/craber-site/build.rs looks at, and it
          # exists for exactly this. It has to be absolute:
          # build.rs hands this straight to PathBuf::from, without the
          # manifest.join() it applies to its other two candidates.
          env.CRABER_WEB_DIST = craber-web;

          # `sqlite-bundled` compiles SQLite and `ring` compiles C and asm, both
          # through the cc crate, and stdenv's compiler is all either wants. No
          # pkg-config and no OpenSSL: sqlx is pinned to rustls-ring, which is
          # what the comment in the root Cargo.toml is protecting.
          buildInputs = lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.libiconv
          ];

          # Three reasons, none of them "tests are slow". The craber-core tests
          # write generated TypeScript back into web/src/types through
          # TS_RS_EXPORT_DIR, the craber-introspect tests are #[ignore]d because
          # they want Docker, and CI runs both properly already.
          doCheck = false;

          # A missing bundle is deliberately not a build error — build.rs only
          # warns and the binary refuses at run time. So without
          # this, a wrong CRABER_WEB_DIST yields a derivation that builds,
          # installs, and cannot do half its job. crates/craber/tests/build.rs
          # cannot stand in for it either: it calls bundle_or_skip() and passes
          # by skipping.
          doInstallCheck = true;
          installCheckPhase = ''
            runHook preInstallCheck
            $out/bin/craber build --file fixtures/tiny.sql -o "$TMPDIR/site"
            test -s "$TMPDIR/site/index.html"
            test -s "$TMPDIR/site/schema.json"
            runHook postInstallCheck
          '';

          meta = {
            description = "CrabER: visualize ER diagrams from SQL DDL or a live database";
            homepage = "https://github.com/kobayashiyabako16g/craber";
            license = lib.licenses.mit;
            mainProgram = "craber";
          };
        };

        rustTooling = with pkgs; [
          rustToolchain
          pkg-config
          openssl
          cargo-watch
          cargo-edit
          cargo-insta
          # Drives the release workflow; dist-workspace.toml pins the same
          # version, and `dist generate` must be run with it.
          cargo-dist
        ];

        # The web app is Vite + React + TypeScript, driven by pnpm. The pnpm
        # here must stay in step with web/package.json's `packageManager` pin.
        webTooling = with pkgs; [
          nodejs_26
          pnpm_11
          typescript
          typescript-language-server
          # JSON/CSS/HTML/ESLint servers, for editors opening web/.
          vscode-langservers-extracted
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs =
            rustTooling
            ++ webTooling
            ++ [ pkgs.just ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
              pkgs.apple-sdk_15
              pkgs.libiconv
            ];

          shellHook = ''
            echo "CrabER development environment"
            echo "Rust: $(rustc --version)"
            echo "Cargo: $(cargo --version)"
            echo "Node: $(node --version)"
            echo "pnpm: $(pnpm --version)"
            echo "TypeScript: $(tsc --version)"
          '';

          # Environment variables
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
        };

        packages = {
          default = craber;
          inherit craber craber-web;
        };

        apps.default = flake-utils.lib.mkApp { drv = craber; };

        checks = {
          inherit craber craber-web;
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
