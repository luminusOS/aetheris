# AGENTS Instructions

## Git Policy

Never create commits, pull requests, tags, or push to any remote unless the user explicitly asks for that operation. Leave release tagging and publishing to the user.

Do not revert unrelated changes in the working tree. This repository often has active UI, packaging, and documentation work in progress.

## No Real-World Identifiers

Never put real cluster names, namespace names, server hostnames, organization names, or other identifiers from an actual environment into code, comments, tests, or fixtures — including ones seen in bug reports, logs, or a real kubeconfig while debugging. This applies everywhere, not just test data: examples, placeholder text, error messages, sample YAML.

Always use clearly generic/fake values instead — `example.com`, `my-namespace`, `prod`, `console.example.com`, `payroll-hml` are fine; a real customer/org name or an actual internal hostname is not. If a bug reproduction naturally involves a real-looking name, invent a made-up equivalent before it lands in a commit.

## Validation After Changes

Use the smallest validation that proves the change.

- For Rust source changes under `crates/`, run:

```sh
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- For a quick compile check during implementation, run:

```sh
cargo check -p aetheris-app
cargo check -p aetheris-kube
```

- For documentation-only changes, do not run the full Rust suite unless the change touches commands, workflows, manifests, or examples that need validation.
- For Flatpak manifest or release workflow changes, validate the JSON and inspect the workflow shell carefully. Run Rust checks if the workflow depends on Rust build behavior.

Do not leave the task with failing checks unless you clearly explain the blocker and the exact command that failed.

## Common Commands

- Run the app:

```sh
RUST_LOG=aetheris=debug,aetheris_kube=debug cargo run --bin aetheris
```

- Run with an isolated kubeconfig:

```sh
KUBECONFIG=/path/to/kubeconfig.yaml RUST_LOG=aetheris=debug,aetheris_kube=debug cargo run --bin aetheris
```

- Verify formatting:

```sh
cargo fmt --check
```

- Lint:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

- Test:

```sh
cargo test --workspace
```

## Repository Structure

- `crates/aetheris-kube/` — Kubernetes backend with no GTK dependencies.
  - `types.rs` — public DTOs and resource metadata.
  - `manager.rs` / `session.rs` — kubeconfig loading, context summaries, and active client sessions.
  - `kubeconfig.rs` — add/import kubeconfig entries.
  - `resources.rs` / `objects.rs` — discovery, list/watch, summaries, details, related pods.
  - `logs.rs`, `exec.rs`, `portforward.rs` — streaming operations.
  - `mutations.rs` — apply/create/delete/scale/cordon/drain.
  - `metrics.rs`, `events.rs`, `cluster.rs`, `status.rs` — supporting domain behavior.
- `crates/aetheris-app/` — Relm4/GTK4/Libadwaita application.
  - `app.rs` — component state and message definitions.
  - `app/component.rs` — widget construction and signal wiring.
  - `app/handler.rs` — `AppMsg` handling.
  - `app/methods.rs` — state helpers and UI synchronization.
  - `app/widgets.rs`, `app/layout.rs`, `app/dialogs.rs`, `app/object_detail.rs` — UI builders.
  - `app/commands.rs`, `app/streams.rs` — async command bridges to `aetheris-kube`.
  - `app/projects.rs` — project persistence, filters, visible columns, and table widths.
  - `app/yaml.rs`, `app/ansi.rs`, `app/utils.rs` — focused helpers.
- `data/` — desktop file, AppStream metadata, icons.
- `build-aux/` — Flatpak manifest.
- `.github/workflows/` — CI and release automation.

## Architecture Rules

1. Keep `aetheris-kube` free of GTK/Relm4 types. It should stay usable and testable as a pure Kubernetes library.
2. Keep UI state changes in `handler.rs`/`methods.rs`; keep widget construction in `component.rs`, `layout.rs`, `widgets.rs`, and detail/dialog modules.
3. Use structured Kubernetes APIs (`kube`, `k8s-openapi`, `DynamicObject`, `ApiResource`) instead of ad hoc shelling out to `kubectl`.
4. Treat `ProjectStore` as the source of truth for what clusters appear in a project. The kubeconfig is used for connection data, not for automatically assigning contexts to projects.
5. Do not expose secrets in the UI. Token fields are intentionally not pre-filled when editing clusters.
6. Keep long-running Kubernetes streams cancellable. Store and clear abort handles for watches, logs, port-forwarding, and terminals.
7. Prefer clear, actionable error messages. RBAC failures should explain the denied resource/action when possible.

## UI Guidelines

- Follow GNOME HIG and Libadwaita conventions.
- Avoid landing pages when the app can show the real workspace.
- Use header buttons, popovers, toasts, `Adw.Dialog`/`AlertDialog`, and sidebar/list patterns consistently.
- Keep layouts adaptive. Avoid fixed widths that force `AdwToastOverlay` or window content beyond available size.
- Use symbolic icons from the current icon theme when possible.
- Keep row/header alignment stable. Table-like `ListBox` rows must use the same width calculations as their headers.

## Packaging And Release

Primary release packaging is Flatpak, AppImage, macOS DMG, and Windows bundles.
The release workflow runs on version tags (`v1.0.0`, `v1.0.0-rc1`) and
publishes:

- `aetheris-<version>.flatpak`
- `aetheris-<version>-x86_64.AppImage`
- `aetheris-<version>-macos-aarch64.dmg`
- `aetheris-<version>-macos-x86_64.dmg`
- `aetheris-<version>-windows-x86_64.zip`
- `aetheris-<version>-windows-x86_64-setup.exe`
- `aetheris-<version>-source.zip`

Tag versions must match `[workspace.package].version` in `Cargo.toml` without the leading `v` and without any `-rc` suffix.

## Human Review Checklist

Before finishing a change, check:

- public behavior matches UI labels and documentation;
- no secret/token is logged or displayed;
- no unrelated files were reformatted;
- warnings are not introduced;
- user-facing strings are concise and GNOME-style;
- docs and workflows describe behavior that is actually implemented.

## Graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, use the installed graphify skill or instructions before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
