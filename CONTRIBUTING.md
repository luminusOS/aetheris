# Contributing to Aetheris

Thank you for helping improve Aetheris. This project is a native GNOME Kubernetes client written in Rust with GTK4, Libadwaita, Relm4, and kube-rs.

## Development Setup

Install system dependencies on Fedora or inside the project toolbox:

```sh
sudo dnf install -y \
  rust cargo pkgconf-pkg-config \
  gtk4-devel libadwaita-devel gtksourceview5-devel vte291-gtk4-devel \
  openssl-devel
```

Run the app:

```sh
RUST_LOG=aetheris=debug,aetheris_kube=debug cargo run --bin aetheris
```

Run against a specific kubeconfig:

```sh
KUBECONFIG=/path/to/kubeconfig.yaml RUST_LOG=aetheris=debug,aetheris_kube=debug cargo run --bin aetheris
```

## Verification

Before opening a PR or handing off a change, run:

```sh
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Use `cargo check -p aetheris-app` or `cargo check -p aetheris-kube` while iterating.

For documentation-only changes, a full Rust test run is usually not necessary. For workflow or Flatpak manifest changes, validate the affected file format and inspect shell scripts carefully.

## Project Architecture

Aetheris has two Rust crates:

- `aetheris-kube` — pure Kubernetes backend. It must not depend on GTK, Relm4, Libadwaita, or VTE.
- `aetheris-app` — Relm4/GTK4 application. It owns UI state, widgets, project persistence, and command wiring.

See [ARCHITECTURE.md](ARCHITECTURE.md) for diagrams and module responsibilities.

## Coding Guidelines

- Keep backend behavior in `aetheris-kube` and UI behavior in `aetheris-app`.
- Prefer typed Kubernetes APIs and kube-rs primitives over shelling out to `kubectl`.
- Use `anyhow::Context` to make failures actionable.
- Keep async streams cancellable.
- Avoid large multipurpose methods. Move focused behavior into the existing module that owns it.
- Do not log bearer tokens, kubeconfig secrets, or certificate data.
- Preserve user/project state in `ProjectStore`; do not auto-add external kubeconfig contexts to a project.

## UI Guidelines

- Follow GNOME HIG and Libadwaita conventions.
- Use symbolic icons for header buttons and actions.
- Keep table headers and rows aligned. If a table column width changes, header and rows must share the same width source.
- Keep dialogs focused and avoid duplicating full pages inside dialogs.
- Use toasts for transient feedback and clear inline messages for errors that block progress.
- Keep text concise and truthful. Do not describe a capability that is not implemented.

## Working With Kubernetes Behavior

RBAC varies a lot between clusters. When adding or changing Kubernetes operations:

- handle `Forbidden` errors clearly;
- keep cluster-scoped and namespace-scoped resources distinct;
- prefer best-effort optional data for metrics/events/logs instead of failing the whole detail view;
- test with a restricted kubeconfig when possible;
- do not assume metrics-server is installed.

Useful manual checks:

```sh
kubectl auth can-i list namespaces
kubectl auth can-i list pods -n <namespace>
kubectl auth can-i create pods/exec -n <namespace>
kubectl auth can-i create pods/portforward -n <namespace>
```

## Flatpak And Releases

The Flatpak manifest lives at `build-aux/org.luminusos.Aetheris.json`.
It builds the VTE GTK4 library inside the Flatpak environment, so installing
`vte291-gtk4` on the host does not satisfy the Flatpak build dependency.
The manifest disables automatic AppStream compose during local bundle builds;
validate `data/org.luminusos.Aetheris.metainfo.xml` separately when changing
application metadata.

To build a local Flatpak bundle:

```sh
sudo dnf install flatpak flatpak-builder
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

mkdir -p target/flatpak
flatpak_arch="$(uname -m)"
flatpak-builder \
  --user \
  --force-clean \
  --disable-rofiles-fuse \
  --install-deps-from=flathub \
  --repo=target/flatpak/repo \
  target/flatpak/build \
  build-aux/org.luminusos.Aetheris.json

flatpak build-bundle \
  target/flatpak/repo \
  "target/flatpak/aetheris-dev-linux-${flatpak_arch}.flatpak" \
  org.luminusos.Aetheris \
  stable
```

Install and run it with:

```sh
flatpak install --user --reinstall "target/flatpak/aetheris-dev-linux-$(uname -m).flatpak"
flatpak run org.luminusos.Aetheris
```

To build a local AppImage, install the native GTK build dependencies,
`cargo-appimage`, and `appimagetool`:

```sh
sudo dnf install -y \
  rust cargo curl file patchelf pkgconf-pkg-config \
  gtk4-devel libadwaita-devel gtksourceview5-devel vte291-gtk4-devel \
  openssl-devel desktop-file-utils appstream

cargo install cargo-appimage
mkdir -p ~/.local/bin
curl -fL \
  https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage \
  -o ~/.local/bin/appimagetool-bin
mkdir -p ~/.local/share/appimagetool
curl -fL \
  https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64 \
  -o ~/.local/share/appimagetool/runtime-x86_64
chmod +x ~/.local/bin/appimagetool-bin ~/.local/share/appimagetool/runtime-x86_64
cat > ~/.local/bin/appimagetool <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec "$HOME/.local/bin/appimagetool-bin" --runtime-file "$HOME/.local/share/appimagetool/runtime-x86_64" "$@"
EOF
chmod +x ~/.local/bin/appimagetool

cd crates/aetheris-app
APPIMAGE_EXTRACT_AND_RUN=1 cargo appimage --locked
```

`cargo-appimage` reads `[package.metadata.appimage]` from
`crates/aetheris-app/Cargo.toml`, so run it from that directory instead of the
workspace root.

Release tags use the workspace version:

```sh
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

Release candidates use a suffix:

```sh
git tag -a v1.0.0-rc1 -m "Release v1.0.0-rc1"
git push origin v1.0.0-rc1
```

The release workflow validates the tag against `[workspace.package].version`
in `Cargo.toml` and `[package].version` in `crates/aetheris-app/Cargo.toml`,
runs CI, builds Flatpak, AppImage, macOS `.dmg`, Windows portable `.zip`, and
Windows setup `.exe` bundles, and publishes the artifacts to GitHub Releases.
GitHub provides the release source archives automatically. The macOS bundles are
generated with `cargo-bundle`, Homebrew GTK runtime dylibs, and `create-dmg`.
The Windows portable bundle contains `bin/aetheris.exe` plus the GTK runtime
files it needs, and the setup `.exe` is generated with Inno Setup.

## Pull Request Checklist

- The change is scoped to one concern.
- `cargo fmt`, clippy, and tests pass when code changes are involved.
- UI changes follow GNOME HIG and remain adaptive.
- New user-facing behavior has useful errors and sensible empty states.
- Secrets are not displayed or logged.
- Documentation is updated when behavior, packaging, or commands change.
- No stale references to old project names remain.
