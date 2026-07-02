<div align="center">

<img src="data/icons/hicolor/scalable/apps/org.luminusos.Aetheris.svg" alt="Aetheris logo" width="160" height="160" />

# Aetheris

**Your clusters, above the clouds.**

A native GNOME Kubernetes client, built in Rust with GTK4 and Libadwaita.

[Website](https://luminusos.org/aetheris) · [LuminusOS](https://luminusos.org) · [Report a bug](https://github.com/luminusOS/aetheris/issues)

</div>

---

Aetheris takes its name from *Aether* — in classical mythology, the highest,
purest and brightest layer of the sky. That is the idea behind the app: a
place where your clusters run clean, clear and untouchable.

It connects through kubeconfig, organizes clusters by project, and provides a
desktop UI for browsing resources, inspecting YAML, streaming pod logs, opening
interactive pod terminals, and running common operations such as apply, delete,
scale, cordon, drain, and port forwarding.

## Features

- **Projects & clusters** — organize any number of clusters into projects and switch instantly.
- **Resource browser** — workloads, networking, storage and config across all namespaces, with live status.
- **YAML editor** — inspect and edit any object with syntax highlighting, then apply it back.
- **Live logs** — real-time pod log streaming with follow mode and ANSI colors.
- **Pod terminals** — a real interactive terminal inside any container, powered by VTE.
- **Operations** — scale, delete, cordon, drain and port-forward without leaving the app.
- **Kubeconfig-first** — reads `~/.kube/config`, and can import and create entries.

## Screenshots

> Aetheris is under active development. Screenshots land here as the UI settles —
> drop them in `data/screenshots/` and swap the placeholders below.

| Cluster overview | Resource browser |
| :---: | :---: |
| <!-- <img src="data/screenshots/overview.png" alt="Cluster overview" /> --> *coming soon* | <!-- <img src="data/screenshots/resources.png" alt="Resource browser" /> --> *coming soon* |

| YAML editor | Logs & pod terminal |
| :---: | :---: |
| <!-- <img src="data/screenshots/yaml.png" alt="YAML editor" /> --> *coming soon* | <!-- <img src="data/screenshots/terminal.png" alt="Logs and pod terminal" /> --> *coming soon* |

## Requirements

On Fedora or inside the project toolbox:

```sh
sudo dnf install -y rust cargo pkgconf-pkg-config gtk4-devel libadwaita-devel gtksourceview5-devel vte291-gtk4-devel openssl-devel
```

## Run

```sh
RUST_LOG=aetheris=debug,aetheris_kube=debug cargo run --bin aetheris
```

By default Aetheris reads and writes `~/.kube/config`. You can point it at a
specific kubeconfig while testing:

```sh
KUBECONFIG=/path/to/kubeconfig.yaml RUST_LOG=aetheris=debug,aetheris_kube=debug cargo run --bin aetheris
```

## Verify

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Flatpak

The Flatpak manifest lives at `build-aux/org.luminusos.Aetheris.json`.

The manifest grants network access and access to `~/.kube` because Aetheris can
read, import, and create kubeconfig entries. Offline Flatpak builds require a
generated Cargo source manifest before release packaging.

## Built with

[Rust](https://www.rust-lang.org/) · [GTK4](https://gtk.org/) ·
[Libadwaita](https://gitlab.gnome.org/GNOME/libadwaita) ·
[Relm4](https://relm4.org/) · [kube-rs](https://kube.rs/) ·
[GtkSourceView](https://gitlab.gnome.org/GNOME/gtksourceview) · [VTE](https://gitlab.gnome.org/GNOME/vte)
