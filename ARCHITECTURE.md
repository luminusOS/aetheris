# Aetheris Architecture

Aetheris is a native GNOME Kubernetes client. It is split into a pure Kubernetes backend crate and a Relm4/GTK4/Libadwaita application crate.

## Overview

```mermaid
flowchart TD
  User[User] --> UI[Aetheris GTK UI]
  UI --> Relm[Relm4 App Component]
  Relm --> Commands[Async Commands and Streams]
  Commands --> KubeCrate[aetheris-kube]
  Commands -. optional VPN .-> OpenConnectCrate[aetheris-openconnect]
  KubeCrate --> Kubeconfig[Kubeconfig]
  KubeCrate --> Api[Kubernetes / OpenShift API]
  Relm --> Store[ProjectStore]
  Store --> ProjectsJson[~/.config/aetheris/projects.json]
  UI --> VTE[VTE Terminal Windows]
  VTE --> Commands
```

`aetheris-app` owns windows, widgets, user state, and persistence of Aetheris projects. `aetheris-kube` owns kubeconfig parsing, Kubernetes clients, discovery, list/watch, mutations, logs, exec, port-forwarding, metrics, and resource details. `aetheris-openconnect` is an optional native integration crate for libopenconnect; it isolates C FFI and exposes a Rust API for future VPN workflows.

## Crate Boundaries

```mermaid
flowchart LR
  subgraph App["aetheris-app"]
    State[App state and AppMsg]
    Widgets[GTK/Adwaita widgets]
    Commands[commands.rs]
    Streams[streams.rs]
    Projects[ProjectStore]
  end

  subgraph Kube["aetheris-kube"]
    Manager[KubeManager]
    Session[KubeSession]
    Resources[Discovery and resources]
    Objects[List/watch/details]
    Ops[Logs exec port-forward mutations]
  end

  subgraph OpenConnect["aetheris-openconnect"]
    SafeApi[Safe Rust API]
    Ffi[libopenconnect FFI]
  end

  Widgets --> State
  State --> Commands
  State --> Streams
  Commands --> Manager
  Commands -. optional .-> SafeApi
  SafeApi --> Ffi
  Streams --> Manager
  Manager --> Session
  Session --> Resources
  Session --> Objects
  Session --> Ops
  Projects --> State
```

The backend crate must not import GTK, Adwaita, Relm4, VTE, or application widgets. Shared data crosses the boundary through DTOs exported from `aetheris-kube::types`. Native C integrations are kept out of both UI widgets and `aetheris-kube`; each integration gets a focused crate such as `aetheris-openconnect`, with `unsafe` declarations confined to its FFI module.

## Application Lifecycle

```mermaid
sequenceDiagram
  participant Main as main.rs
  participant App as Relm4 App
  participant Cmd as commands.rs
  participant Kube as KubeManager
  participant Store as ProjectStore
  participant UI as GTK UI

  Main->>App: launch org.luminusos.Aetheris
  App->>Cmd: load_state()
  Cmd->>Kube: load kubeconfig
  Kube-->>Cmd: contexts + namespaces
  Cmd->>Store: load projects.json and normalize known contexts
  Cmd-->>App: LoadedState
  App->>UI: show projects page
  UI->>App: project selected
  App->>UI: show clusters page
  App->>Cmd: load_cluster_summary(context)
  Cmd-->>App: ClusterSummaryLoaded
```

The app starts on the Projects page. Selecting a project shows only clusters explicitly assigned to that project. Contexts created externally by `kubectl` or `oc` do not automatically appear in projects.

## Cluster And Resource Flow

```mermaid
sequenceDiagram
  participant UI as Resource UI
  participant App as App handler
  participant Cmd as commands.rs
  participant Kube as KubeSession
  participant Watch as kube runtime watcher

  UI->>App: ClusterChanged
  App->>Cmd: load_cluster(context)
  Cmd->>Kube: connect_context
  Kube->>Kube: list_namespaces + discover_resources
  Cmd-->>App: ClusterLoaded
  App->>Cmd: list_objects(context, resource, namespace)
  Cmd->>Kube: list_objects
  Kube-->>App: ObjectsLoaded snapshot
  App->>Cmd: stream_object_watch
  Cmd->>Watch: watch selected resource
  Watch-->>App: Restarted / Applied / Deleted / Error
```

The list view uses a snapshot for fast initial rendering, then a watcher keeps visible objects current. Rows are built in chunks to avoid blocking the GTK main loop on very large resource lists.

## Details And Operations

```mermaid
flowchart TD
  ObjectRow[Object row activation] --> Detail[object_detail]
  Detail --> Overview[Overview]
  Detail --> YAML[YAML with SourceView]
  Detail --> Events[Events]
  Detail --> Conditions[Conditions]
  Detail --> Related[Related Pods for Deployments]
  Detail --> Logs[Pod logs]
  Detail --> Containers[Pod containers and metrics]

  YAML --> Apply[Server-side apply]
  Overview --> Scale[Scale Deployment]
  Overview --> NodeOps[Cordon / Drain Node]
  Detail --> Delete[Delete Object]
  Logs --> LogStream[Log stream]
  Containers --> Terminal[VTE pod terminal]
  Overview --> PortForward[Pod port-forward]
```

Operations run through `aetheris-kube` and return updated details or explicit errors. Long-running operations use abort handles so switching clusters, closing windows, or changing detail views can stop background work cleanly.

## Terminal Flow

```mermaid
sequenceDiagram
  participant Win as Terminal Window
  participant App as App streams
  participant Kube as terminal_pod
  participant Api as Kubernetes pods/exec

  Win->>App: open terminal for Pod
  App->>Kube: PodExecRequest + input channel
  Kube->>Api: exec sh/bash with TTY
  Api-->>Kube: stdout stream
  Kube-->>App: PodExecEvent
  App-->>Win: feed VTE
  Win-->>App: user input
  App-->>Kube: stdin bytes
  Api-->>Kube: status or RBAC error
  Kube-->>App: finished result
```

The default terminal container is selected from the Pod name when possible. If Kubernetes denies `pods/exec`, the terminal window displays a permission error instead of staying blank.
Windows release bundles currently compile without VTE because the GTK4 VTE
library is not available in the MSYS2/MinGW package set used by the CI; Pod
terminal windows are therefore disabled in that build.

## Project Store

```mermaid
classDiagram
  class ProjectStore {
    projects Vec~Project~
    selected_project Option~String~
    visible_object_columns Vec~ObjectColumn~
    object_name_width Option~i32~
    object_column_widths Vec~ObjectColumnWidth~
  }

  class Project {
    name String
    contexts Vec~String~
    custom_namespaces Vec~String~
  }

  class Kubeconfig {
    contexts
    clusters
    users
  }

  ProjectStore --> Project
  ProjectStore ..> Kubeconfig : prunes missing contexts only
```

`ProjectStore` lives in `~/.config/aetheris/projects.json`. It controls which clusters appear in each project, custom namespaces, visible columns, and table widths. Kubeconfig contexts are not automatically imported into projects.

## Packaging And Release

```mermaid
flowchart TD
  Tag[vX.Y.Z or vX.Y.Z-rcN] --> ReleaseWorkflow[release.yml]
  ReleaseWorkflow --> CI[ci.yml]
  CI --> RustChecks[fmt + clippy + tests]
  RustChecks --> Flatpak[flatpak-builder]
  Flatpak --> Bundle[aetheris-version.flatpak]
  RustChecks --> AppImage[cargo-appimage]
  AppImage --> AppImageBundle[aetheris-version-x86_64.AppImage]
  RustChecks --> MacOS[cargo-bundle + create-dmg]
  MacOS --> MacDmg[aetheris-version-macos-arch.dmg]
  RustChecks --> Windows[MSYS2 CLANG64]
  Windows --> WindowsBundle[aetheris-version-windows-x86_64.zip]
  Windows --> WindowsSetup[Inno Setup exe]
  ReleaseWorkflow --> Source[git archive source.zip]
  Bundle --> GitHubRelease[GitHub Release]
  AppImageBundle --> GitHubRelease
  MacDmg --> GitHubRelease
  WindowsBundle --> GitHubRelease
  WindowsSetup --> GitHubRelease
  Source --> GitHubRelease
```

The Flatpak manifest is `build-aux/org.luminusos.Aetheris.json`. AppImage
metadata lives in `crates/aetheris-app/Cargo.toml` under
`[package.metadata.appimage]`. macOS bundles use `cargo-bundle`, Homebrew GTK
runtime dylibs copied by `scripts/bundle-macos-runtime.sh`, and `create-dmg`.
The Windows bundle is built with MSYS2/CLANG64 and contains `bin/aetheris.exe`
plus GTK runtime files. The Windows setup installer is generated by Inno Setup
from `build-aux/inno/aetheris.iss`. Releases are created from tags and publish
Flatpak, AppImage, macOS, Windows, and source zip artifacts.

## Design Constraints

- The UI follows GNOME HIG and Libadwaita patterns.
- The resource browser must stay usable on narrow windows and avoid forcing the window wider.
- Errors should be actionable, especially Kubernetes RBAC denials.
- Secrets such as bearer tokens are never re-rendered when editing a cluster.
- Backend modules should remain small and focused; do not grow `lib.rs` beyond module declarations and public exports.
