{
  description = "Rage of Mages (J2ME) decompilation, resource analysis, and native Rust port environment";

  # Pinned to the same rev the sibling J2ME decomp flakes share so the projects
  # use one toolchain closure (bump deliberately, together). Never `path:.` — a
  # git-backed flake ref copies only tracked files into the store, which is why
  # this repository keeps every bulk/derived path git-ignored.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/1559d3daa3ecc813a650b79375ea61b6741b8746";

  outputs =
    { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      runtimeLibraries =
        pkgs: with pkgs; [
          alsa-lib libGL libpulseaudio libxkbcommon udev
          vulkan-loader wayland libx11 libxcursor libxi libxrandr
        ];
    in
    {
      apps = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          # Binary resources live in a PRIVATE resources location (this repo is
          # CC0 and resource-free). The app takes that location explicitly — a
          # local checkout path or a git URL — copies `originals/` into
          # git-ignored `_originals/`, and verifies every payload against
          # builds.toml. No location is ever baked into the repo.
          #   nix run .#fetch-resources -- <path-or-git-url>
          #   RAGE_OF_MAGES_RESOURCES=<same> nix run .#fetch-resources
          fetchResources = {
            type = "app";
            program = toString (
              pkgs.writeShellScript "fetch-resources" ''
                set -euo pipefail
                if [ ! -f game.toml ]; then
                  echo "run from the repository root" >&2
                  exit 1
                fi
                source="''${1:-''${RAGE_OF_MAGES_RESOURCES:-}}"
                if [ -z "$source" ]; then
                  echo "usage: nix run .#fetch-resources -- <checkout-path-or-git-url>" >&2
                  echo "   or: RAGE_OF_MAGES_RESOURCES=<same> nix run .#fetch-resources" >&2
                  exit 1
                fi
                if [ -d "$source" ]; then
                  resources=$source
                else
                  tmp=$(${pkgs.coreutils}/bin/mktemp -d)
                  trap 'rm -rf "$tmp"' EXIT
                  ${pkgs.git}/bin/git clone --quiet --depth 1 "$source" "$tmp/resources"
                  resources=$tmp/resources
                fi
                ${pkgs.python3}/bin/python3 tools/originals/fetch.py "$resources"
              ''
            );
            meta.description = "Materialize and verify the binary resources from an explicit checkout path or git URL";
          };
        in
        { fetch-resources = fetchResources; default = fetchResources; }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          libraries = runtimeLibraries pkgs;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              # Rust implementation and native debugging.
              cargo cargo-nextest clippy rust-analyzer rustc rustfmt
              clang cmake gdb lld mold pkg-config
              # Java ME bytecode inspection and decompilation.
              cfr jadx jdk17_headless
              # In-process audio output (cpal -> libasound via pkg-config).
              alsa-lib
              # Archive, binary, image, and resource inspection.
              binutils diffutils file ffmpeg hexyl imagemagick jq p7zip
              pngcheck python3 ripgrep sox sqlite timidity tree unzip which xxd zip
              # Browser play page and Rust/WASM.
              binaryen chromium firefox geckodriver nodejs pnpm
              wasm-bindgen-cli wasm-pack wasm-tools
              # Workspace utilities.
              git just nixfmt
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
            JAVA_HOME = "${pkgs.jdk17_headless}";
            PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH = "${pkgs.chromium}/bin/chromium";
            PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
            RUST_BACKTRACE = "1";
            shellHook = ''
              echo "Rage of Mages — J2ME recovery environment"
              echo "  bootstrap: just bootstrap <resources>"
              echo "  verify:    just originals-verify"
              echo "  all gates: just check"
            '';
          };
        }
      );
    };
}
