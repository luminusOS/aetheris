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
sudo flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

mkdir -p dist
flatpak-builder \
  --force-clean \
  --install-deps-from=flathub \
  --repo=dist/repo \
  dist/build \
  build-aux/org.luminusos.Aetheris.json

flatpak build-bundle \
  dist/repo \
  dist/aetheris-dev.flatpak \
  org.luminusos.Aetheris \
  stable
```

Install and run it with:

```sh
flatpak install --user --reinstall dist/aetheris-dev.flatpak
flatpak run org.luminusos.Aetheris
```

Release tags use the workspace version:

```sh
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

Release candidates use a suffix:

```sh
git tag -a v0.1.0-rc1 -m "Release v0.1.0-rc1"
git push origin v0.1.0-rc1
```

The release workflow validates the tag against `[workspace.package].version`, runs CI, builds a Flatpak bundle, creates a source zip, and publishes both to GitHub Releases.

## Pull Request Checklist

- The change is scoped to one concern.
- `cargo fmt`, clippy, and tests pass when code changes are involved.
- UI changes follow GNOME HIG and remain adaptive.
- New user-facing behavior has useful errors and sensible empty states.
- Secrets are not displayed or logged.
- Documentation is updated when behavior, packaging, or commands change.
- No stale references to old project names remain.
