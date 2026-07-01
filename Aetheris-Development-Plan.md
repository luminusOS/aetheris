# Aetheris — Cliente Kubernetes nativo (Rust / GTK4 / Libadwaita)

## Context

LuminusOS é um monorepo com apps nativos em Rust/GTK4 (sirius, wedroid). Falta uma
ferramenta para **conectar e operar clusters Kubernetes** com UI nativa GNOME, no
espírito do [getseabird/seabird](https://github.com/getseabird/seabird) (que é em Go).

**Aetheris** será essa app: um cliente Kubernetes desktop em Rust, reaproveitando ao
máximo crates prontas para reduzir boilerplate, seguindo as convenções do monorepo
(Relm4 + Libadwaita, workspace Cargo, manifesto Flatpak JSON, configs de lint do
readymade). Empacotamento primário **Flatpak**, secundário **AppImage**.

Decisões de produto:
- **UI:** Relm4 (consistência com sirius/wedroid).
- **Escopo:** paridade total com Seabird, entregue em fases.
- **Auth:** kubeconfig + exec credential plugins (aws/gcp/oidc).
- **Local/ID:** diretório top-level `aetheris/`, App ID `org.luminusos.Aetheris`, nome **Aetheris**.

## Incremento executado inicialmente

O primeiro incremento implementável é **Fase 0 + base da Fase 1**:
- workspace Rust independente em `aetheris/`, seguindo o padrão de `wedroid/`;
- crates `aetheris-kube` (sem GTK) e `aetheris-app` (Relm4/Libadwaita);
- configs de lint/format copiadas de `readymade`;
- assets mínimos (`.desktop`, AppStream, ícone SVG) e manifesto Flatpak skeleton;
- carregamento de kubeconfig, enumeração de contextos e namespaces conhecidos no
  kubeconfig;
- conexão com contexto selecionado e listagem read-only de Pods do namespace
  selecionado.

Ficam fora deste incremento: discovery genérico, watchers, YAML/detail, logs,
mutações, terminal, port-forward, vendoring Flatpak e AppImage.

O segundo incremento implementado avança a **Fase 2 parcial**:
- `aetheris-kube` usa `kube::discovery::Discovery` para enumerar recursos listáveis
  do cluster selecionado;
- a UI substitui a lista fixa de Pods por uma sidebar dinâmica de kinds;
- a listagem principal usa `Api::<DynamicObject>` para exibir qualquer recurso
  read-only com colunas Name, Namespace, Status, API Version e Age;
- namespaces passam a ser carregados do cluster quando possível, com fallback para
  namespaces presentes no kubeconfig.

Neste ponto ainda ficavam pendentes da Fase 2: watcher/cache vivo com
`kube::runtime`, atualizações incrementais add/update/delete e busca/filtro
textual.

O terceiro incremento continua a **Fase 2** e melhora o primeiro uso:
- tela inicial clean aparece quando não há kubeconfig/contexto carregado, com
  convite central e botão para adicionar cluster;
- o botão abre um diálogo central com opções de conexão;
- cadastro grava um kubeconfig com contexto, cluster e usuário token-based
  (`KUBECONFIG` com caminho único ou `~/.kube/config` como padrão);
- importação de kubeconfig mescla contextos de um arquivo existente no destino
  configurado;
- o browser ganha busca textual local sobre Name, Namespace, Status e API Version;
- o setup aceita API server, bearer token, CA data base64 opcional e opção de
  ignorar verificação TLS.

Ainda ficam pendentes da Fase 2: watcher/cache vivo com `kube::runtime` e
atualizações incrementais add/update/delete.

---

## Stack & crates (minimizar boilerplate)

| Necessidade | Crate | Notas |
|---|---|---|
| Cliente K8s | `kube` (features `client, config, runtime, ws, derive`) | kube-rs; `ws` habilita exec/attach/port-forward |
| Tipos da API | `k8s-openapi` (feature de versão, ex. `latest`) | tipos tipados + `DynamicObject` para CRDs |
| Async | `tokio` (full) | obrigatório pelo kube-rs |
| UI | `relm4` 0.11 + `gtk4` 0.11 + `adw` (libadwaita `gnome_46`) | mesmas versões do wedroid |
| Tabelas de recursos | `relm4::typed_view::column::TypedColumnView` | listas/colunas com mínimo boilerplate |
| Editor YAML | `sourceview5` | highlight de sintaxe no detalhe/edição |
| Terminal (exec) | `vte4` | widget de terminal; ponte PTY ↔ websocket do kube |
| Serialização | `serde`, `serde_json`, `serde_yaml` | YAML/JSON de manifestos |
| Erros/log | `anyhow`, `thiserror`, `tracing`, `tracing-subscriber` | padrão do monorepo |
| Datas/idade | `chrono` (+ `humantime`) | coluna "Age", timestamps |
| Paths | `dirs` | localizar `~/.kube/config` |
| Streams | `futures` | combinar watchers/log streams |

Reaproveitamento de infra que **evita escrever do zero**:
- `kube::runtime::{watcher, reflector, WatchStreamExt}` + `Store` → list/watch e cache vivo, sem polling manual.
- `kube::discovery::Discovery` → enumera GroupVersionKinds (inclui CRDs) → sidebar dinâmica, sem hardcode de kinds.
- `kube::Api::<DynamicObject>` + `ApiResource` → operar qualquer recurso genericamente (como o Seabird).
- `kube::Config::infer()` / `Kubeconfig` → contextos e exec plugins prontos.
- `TypedColumnView` (relm4) → tabela ordenável/filtrável sem boilerplate de `ColumnView` cru.

---

## Arquitetura

Workspace Cargo em `aetheris/` (espelha sirius/wedroid):

```
aetheris/
  Cargo.toml                      # workspace: deps compartilhadas, profile release (lto thin, strip)
  rustfmt.toml, clippy.toml       # copiar de readymade/
  crates/
    aetheris-kube/                  # backend K8s puro (SEM tipos GTK) — testável
      src/{client,discovery,store,ops,logs,exec,portforward,kubeconfig}.rs
    aetheris-app/                   # UI Relm4
      src/{main,app, pages/, dialogs/, widgets/}.rs
  data/
    org.luminusos.Aetheris.desktop
    org.luminusos.Aetheris.metainfo.xml
    icons/hicolor/scalable/apps/org.luminusos.Aetheris.svg
  build-aux/
    org.luminusos.Aetheris.json     # manifesto Flatpak
    cargo-sources.json            # gerado por flatpak-cargo-generator
```

**Separação de responsabilidades (chave):**
- `aetheris-kube` não conhece GTK. Expõe um **handle/ator** (task tokio) que recebe
  *comandos* (listar kind, abrir logs, apply, delete, exec...) e emite *eventos*
  (snapshot inicial, add/update/delete de objeto, linha de log, erro) por canais
  `tokio::sync::mpsc`/`broadcast`.
- `aetheris-app` (Relm4) consome eventos e mapeia para mensagens dos componentes.

**Ponte tokio ↔ Relm4:** Relm4 já roda sobre tokio. Usar `AsyncComponent`/`Worker`
e `Command`/`CommandOutput`; spawnar watchers com `relm4::spawn`/`tokio::spawn` e
enviar para os componentes via `Sender`. Sem GTK fora do main loop.

**Layout da UI (Adwaita), inspirado no Seabird:**
- `adw::ApplicationWindow` + `adw::NavigationSplitView` (ou `OverlaySplitView`).
- **Sidebar:** switcher de cluster/contexto (dropdown no header) + lista de kinds
  agrupada (Workloads, Config, Network, Storage, RBAC, Cluster, Custom/CRDs) + busca.
- **Content:** `TypedColumnView` dos objetos do kind selecionado (colunas: Name,
  Namespace, Status, Age...), filtro por namespace, campo de busca; atualização viva via watcher.
- **Detail:** `NavigationView` push → `ViewStack`/tabs por recurso:
  Overview (campos-chave), YAML (`sourceview5`, leitura→edição), Events,
  Logs (pods, streaming, picker de container), Terminal (exec via `vte4`).
- Ações no header/menu: namespace selector, refresh, create (apply YAML),
  delete/scale/cordon/drain conforme o kind; confirmação via `adw::AlertDialog`;
  feedback via `adw::Toast`.

---

## Plano de execução em fases

Paridade total é grande; entregar incrementalmente e verificável a cada fase.

- **Fase 0 — Scaffolding.** Workspace, dois crates, configs de lint (copiar
  readymade), `main.rs` com `RelmApp::new("org.luminusos.Aetheris")` + tracing,
  janela Adwaita vazia com split view. Esqueleto do manifesto Flatpak.
- **Fase 1 — Conectar.** `aetheris-kube`: carregar kubeconfig, listar/trocar
  contextos (multi-cluster), criar `Client` (incl. exec plugins), `Discovery`.
  UI: dropdown de contexto + lista read-only de Pods de um namespace.
- **Fase 2 — Browser genérico.** Sidebar dinâmica a partir do `Discovery`;
  `Api::<DynamicObject>` por kind; `watcher`/`Store`, add/update/delete vivos,
  `TypedColumnView` e busca ficam como próximos passos.
- **Fase 3 — Detalhe.** Push de detalhe: Overview, YAML (somente leitura via
  `serde_yaml`), aba Events (Events do objeto), colunas Status/Age.
- **Fase 4 — Logs.** Stream de logs de pod (`Api::log_stream`), follow,
  multi-container, parar/limpar.
- **Fase 5 — Mutações.** Editar+aplicar YAML (server-side apply via `Patch::Apply`),
  delete (com confirmação), scale, cordon/drain, create a partir de YAML.
- **Fase 6 — Terminal & port-forward.** Exec em pod: ponte `vte4` PTY ↔ websocket
  `Api::exec`; `Api::portforward` com lista de forwards ativos.
- **Fase 7 — Polish & packaging.** i18n (fluent via `i18n-embed`, padrão readymade;
  começar em EN), toasts de erro, metainfo AppStream, ícone, finalizar Flatpak,
  AppImage, CI (workflow rust + flatpak-builder).

---

## Arquivos/templates a reaproveitar

- Manifesto Flatpak base: `wedroid/build-aux/org.gnome.WeDroid.json`
  (runtime `org.gnome.Platform` 49, `org.freedesktop.Sdk.Extension.rust-stable`,
  build `cargo build --release --offline`).
- Lint: `readymade/clippy.toml`, `readymade/rustfmt.toml` (copiar).
- AppStream exemplo: `wedroid/data/org.gnome.WeDroid.metainfo.xml`.
- Estrutura de workspace/crates: `sirius/Cargo.toml`, `wedroid/Cargo.toml`.
- Profile release (`lto = "thin"`, `strip = true`): de `wedroid`.

---

## Packaging

**Flatpak (foco):** `build-aux/org.luminusos.Aetheris.json`.
- Runtime `org.gnome.Platform` 49 + SDK; extensão `rust-stable`. `sourceview5` e
  `vte` já vêm no runtime GNOME.
- `finish-args`: `--share=network` (acesso aos clusters), `--socket=wayland`/`fallback-x11`,
  `--filesystem=~/.kube` (kubeconfig; rw se for editar contexto), `--filesystem=xdg-config/aetheris:create`.
- **Risco — exec credential plugins no sandbox:** binários como `aws`/`gcloud`/`kubectl`
  ficam no host, não no sandbox. Opções: (a) `--talk-name=org.freedesktop.Flatpak` +
  rodar plugin via `flatpak-spawn --host`; (b) ampliar `--filesystem=host`. Decidir na
  Fase 1; documentar limitação.
- Build offline exige vendoring: gerar `cargo-sources.json` com
  `flatpak-cargo-generator.py` a partir do `Cargo.lock`.

**AppImage (secundário):** `cargo build --release` + `linuxdeploy` com plugin GTK,
empacotando libadwaita/sourceview/vte. GTK em AppImage é trabalhoso; manter como alvo
secundário, validar só após Flatpak estável.

---

## Verification

- **Build/lint:** `cargo build --release` e `cargo clippy --all-targets -- -D warnings`
  passam no workspace; `cargo fmt --check`.
- **Cluster de teste local:** subir `kind` ou `k3s`/`minikube`; apontar `KUBECONFIG`.
  - Fase 1: app lista contextos e Pods do cluster de teste.
  - Fase 2: sidebar mostra kinds (incl. um CRD instalado de teste); criar/deletar um
    Deployment via `kubectl` reflete ao vivo na lista (watcher).
  - Fase 4: `kubectl run` de um pod que loga em loop → logs aparecem em streaming.
  - Fase 5: editar YAML e aplicar; scale de Deployment; confirmar via `kubectl get`.
  - Fase 6: exec abre shell interativo num pod; port-forward acessível via `curl localhost`.
- **Testes unitários** em `aetheris-kube` (parsing de kubeconfig, mapeamento
  discovery→kinds) sem precisar de cluster (mockar onde possível).
- **Flatpak:** `flatpak-builder --user --install build build-aux/org.luminusos.Aetheris.json`
  builda e a app abre conectando ao cluster de teste.

---

## Out of scope (v1)

- Helm / gerenciamento de charts.
- Métricas/gráficos (CPU/memória) — possível fase futura via metrics-server.
- Edição visual de RBAC ou wizards de criação (usar YAML).
