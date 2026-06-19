# Data Specs

Data Specs is a markdown-first data transformation framework. You define models, transformations, tests, operations, and templates in plain `.md` files. A Rust compiler turns those specs into a typed entity catalog at build time; a generated binary runs that catalog against your warehouse at runtime.

---

## Two tools, two roles

| Tool | When | What it does |
|------|------|--------------|
| **`dataspec` CLI** | Once, to bootstrap | Scaffolds a new Rust project with `data-specs/` and wiring |
| **Project binary** | Day to day | Looks up entities in the catalog and runs SQL via `transform` / `list` |

The `dataspec` binary does **not** parse specs or execute SQL. That happens inside each generated project: `build.rs` compiles specs, the binary runs them.

---

## Quick start

### 1. Build the tool

```bash
cargo build --release --features bq,pg
```

### 2. Create a project

```bash
cargo run -- new my_pipeline
cd ../my_pipeline
```

This creates:

```
my_pipeline/
├── Cargo.toml
├── build.rs                 # calls spec_builder at compile time
├── data-specs/              # your markdown specs (source of truth)
│   ├── config/config.md
│   ├── models/
│   ├── transformations/
│   ├── templates/
│   ├── operations/
│   └── tests/
└── src/
    ├── main.rs              # loads catalog, runs spec_handler CLI
    ├── lib.rs
    └── data.rs              # generated — do not edit (gitignored)
```

### 3. Build and run

```bash
cargo build
cargo run -- transform --names dummy_model
cargo run -- list --models
```

On `cargo build`, `build.rs` walks `data-specs/**/*.md`, parses them, and writes `src/data.rs` with static entities and a `register_data()` function.

---

## How it works

```
┌─────────────────────────────────────────────────────────────────┐
│  data-specs/**/*.md                                             │
└────────────────────────────┬────────────────────────────────────┘
                             │  cargo build
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  build.rs  →  dataspec::spec_builder("data-specs", "src/data.rs")│
│               parse · validate · emit Rust source                │
└────────────────────────────┬────────────────────────────────────┘
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  src/data.rs  (generated)                                       │
│    LazyLock<Model>, LazyLock<Transformation>, …                 │
│    register_data() -> DataCatalog                               │
└────────────────────────────┬────────────────────────────────────┘
                             │  cargo run
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  main.rs  →  catalog = data::register_data()                    │
│              dataspec::spec_handler(&catalog)                   │
│                transform / list  →  Engine  →  warehouse        │
└─────────────────────────────────────────────────────────────────┘
```

### Build time — `spec_builder`

Called from the generated project's `build.rs`:

```rust
fn main() {
    dataspec::spec_builder("data-specs", "src/data.rs")
        .expect("failed to build data catalog");
}
```

`spec_builder`:

1. Parses every `.md` file under `data-specs/`
2. Validates (one config, no duplicate entity names)
3. Emits `src/data.rs` with `LazyLock` statics for each entity
4. Prints `cargo:rerun-if-changed` for the specs directory

Embedded model transformations are emitted as `{model}__default_transformation`.

### Runtime — `spec_handler`

Called from the generated project's `main.rs`:

```rust
#[tokio::main]
async fn main() {
    let catalog = data::register_data();
    dataspec::spec_handler(&catalog).await;
}
```

`spec_handler` is the CLI for the **project binary**, not the `dataspec` scaffolding tool.

---

## Project binary CLI

After `cargo build`, run the project binary (same name as the crate):

### Transform

Run transformations for models by name or tag:

```bash
# Single model (uses default transformation)
cargo run -- transform --names dummy_model

# Explicit transformation
cargo run -- transform --names dummy_model::my_transformation_v2

# By tags
cargo run -- transform --tags core,reporting

# Runtime variables and table mappings
cargo run -- transform --names my_model \
  --vars report_year=2024 \
  --mappings my_model=dataset.table_id

# JSON output
cargo run -- transform --names dummy_model --json
```

### List

Inspect catalog contents:

```bash
cargo run -- list --models
cargo run -- list --operations
cargo run -- list --transformations
cargo run -- list --templates
cargo run -- list --tests

# By name
cargo run -- list --names dummy_model --models

# JSON
cargo run -- list --models --json
```

---

## Storage backends

Set `provider` in `data-specs/config/config.md`:

| `provider` | Description |
|------------|-------------|
| `dryrun` | Default. Logs SQL, no warehouse call |
| `bq` | Google BigQuery (requires `project_id`; optional `service_account_path`) |
| `pg` / `postgres` | PostgreSQL (requires `connection_string`) |

Generated projects depend on `dataspec` with `features = ["bq", "pg"]`. Use `dryrun` for local development without credentials.

Example config:

```markdown
# config

## Type
config

| Key | Value | Description |
| --- | --- | --- |
| `provider` | `dryrun` | Storage backend |
| `project_id` | `my-gcp-project` | BigQuery project (when provider=bq) |
```

---

## Spec format

Specs are Markdown files with a fixed heading structure. Each file describes one entity; the `## Type` section declares its kind (`model`, `transformation`, `template`, `test`, `operation`, or `config`).

See [../specs/README.md](../specs/README.md) for the full format reference, [../specs/data-specs/](../specs/data-specs/) for minimal examples, and [../specs/examples/eth/](../specs/examples/eth/) for a realistic dependency graph.

---

## Scaffolding CLI reference

```bash
dataspec new <name> [--path DIR] [--dataspec-path PATH]
```

| Flag | Description |
|------|-------------|
| `--path` | Directory to create the project in (default: current directory) |
| `--dataspec-path` | Path to this crate for generated project `Cargo.toml` dependencies |

---

## Library API

This crate is a library used by generated projects. Main entry points:

| Function | Used in | Purpose |
|----------|---------|---------|
| `spec_builder(specs_dir, output_path)` | `build.rs` | Parse specs, generate `data.rs` |
| `spec_handler(catalog)` | `main.rs` | Runtime CLI (`transform`, `list`) |

Other exports: `DataCatalog`, entity types (`Model`, `Transformation`, …), `parse_spec_file`, `parse_spec_dir` for programmatic parsing.

---

## Crate layout

```
src/
├── build/         spec_builder, codegen (md → data.rs)
├── parser/        markdown → entities
├── handler.rs     spec_handler (runtime CLI)
├── engines/       dryrun, BigQuery, PostgreSQL
├── scaffold/      dataspec new
├── entities/      Model, Transformation, DataCatalog, …
└── main.rs        scaffolding CLI only (new)
```

---
## Next
- External modules (metadata.data_modules, dataspec add)
- Hooks execution (pre/post/init)
- SQL dependency graph population
- Template compile-time inlining
- Partition/Clusters parsing
---

## License

Apache-2.0
