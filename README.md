## Data Specs

Data Specs is a markdown-first data transformation framework. You define models, transformations, tests, operations, and templates in plain `.md`
files. A Rust compiler turns those specs into a typed entity catalog at build time; a generated binary runs that catalog against your warehouse
at runtime.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Status: early](https://img.shields.io/badge/status-early--stage-orange)](#)

This repo is the **compiler and runtime**. For the spec format itself, examples, and the "why Markdown instead of YAML+Jinja" case, see
[`Dataspecs/specs`](https://github.com/Dataspecs/specs).

---

## Two tools, two roles

| Tool | When | What it does |
|------|------|--------------|
| **`dataspec` CLI** | Once, to bootstrap | Scaffolds a new Rust project with `data-specs/` and wiring |
| **Project binary** | Day to day | Looks up entities in the catalog and runs SQL via `transform`, `apply`, `test`, or `list` |

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
├── README.md                # project quick start and CLI reference
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

After `cargo build`, run the project binary (same name as the crate).

`transform`, `apply`, and `test` share the same runtime flags: `--names`, `--tags`, `--vars`, `--mappings`, `--debug`, and `--json`. Names and tags select different entity kinds depending on the command (see below). `transform` also accepts `--init` to run init hooks.

### Transform

Run transformations for models by name or tag. When a transformation defines hooks, the executor runs them around the transformation SQL in this order:

1. **Init** — only when `--init` is passed
2. **Pre** — always
3. **Transformation** — the model build SQL
4. **Post** — always

Hooks reference [operations](https://github.com/Dataspecs/specs/blob/main/README.md#operation) with optional prop overrides. Define hooks under `## Hooks` in a transformation spec, or under `### Hooks` in an embedded model transformation. See the [spec format reference](https://github.com/Dataspecs/specs/blob/main/README.md#spec-format-reference) for `Pre`, `Post`, and `Init` sections.

```bash
# Single model (uses default transformation)
cargo run -- transform --names dummy_model

# Explicit transformation
cargo run -- transform --names dummy_model::my_transformation_v2

# Run init hooks (e.g. one-time table setup) before pre/transformation/post
cargo run -- transform --names dummy_model --init

# By tags
cargo run -- transform --tags core,reporting

# Runtime variables and table mappings
cargo run -- transform --names my_model \
  --vars report_year=2024 \
  --mappings my_model=dataset.table_id

# JSON output
cargo run -- transform --names dummy_model --json
```

Example hooks in a transformation spec:

```markdown
## Hooks
### Pre
- [eth_set_block_range](../operations/eth_set_block_range)
    | Key | Value | Description |
    | --- | --- | --- |
    | `start_block` | `{{props__eth_default_start_block}}` | Inclusive lower bound |
    | `end_block` | `999999999` | Inclusive upper bound |
### Post
- [eth_update_watermark](../operations/eth_update_watermark)
    | Key | Value | Description |
    | --- | --- | --- |
    | `model_name` | `eth_blocks` | Model name |
    | `end_block` | `{{props__end_block}}` | Last processed block |
### Init
- [create_watermark_table](../operations/create_watermark_table)
```

Hook operations use SQL compiled at build time into each hook reference (`OperationUsage.sql_code`). At execution time only runtime variables are resolved (see [Rendering rules](#rendering-rules)).

### Apply

Run operations by name or tag:

```bash
# Single operation
cargo run -- apply --names dummy_operation

# By tags
cargo run -- apply --tags maintenance

# Runtime variables and table mappings
cargo run -- apply --names dummy_operation \
  --vars report_year=2024 \
  --mappings dummy_model=dataset.table_id

# JSON output
cargo run -- apply --names dummy_operation --json
```

### Test

Run tests linked to models (from the default or explicit transformation, plus column-level tests):

```bash
# Single model (uses default transformation)
cargo run -- test --names dummy_model

# Explicit transformation
cargo run -- test --names dummy_model::my_transformation_v2

# By tags
cargo run -- test --tags core,reporting

# Runtime variables and table mappings
cargo run -- test --names dummy_model \
  --vars report_year=2024 \
  --mappings dummy_model=dataset.table_id

# JSON output
cargo run -- test --names dummy_model --json
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
### 3. Run with the Specs Executor

Specs are compiled into a Rust binary. Use it as a CLI to run transformations, tests, and operations against your storage backend. On large projects with many models, execution performance stays close to executing SQL directly from application code.

<details>
<summary>Rendering rules</summary>

Two rendering phases: **compilation** and **execution**.

#### Compilation-time rendering

- Renders all `{{props}}` and `{{self}}` variables
- Inlines templates into models, operations, and tests
- After rendering, FROM clauses should reference only models or table names
- SQL analysis runs to compute dependencies

#### Execution-time rendering

- Resolves model references to table names (with optional `--mappings`)
- Substitutes `{{vars}}` passed via CLI
- Produces final executable SQL

#### Rendering context

| Variable | When | Description |
|----------|------|-------------|
| `{{props__<key>}}` | Compile (hook / test usage) | Config and template params; hook/test prop overrides — baked into `OperationUsage.sql_code` / `TestUsage.sql_code`. Config props may resolve earlier during template inlining |
| `{{model.name}}`, `{{model.tags}}`, `{{model.description}}`, `{{model.managed}}`, `{{model.disabled}}`, `{{model.meta}}` | Compile (hook / test usage) | Model metadata from the transformation's model; preserved through template inlining until usage compile |
| `{{model.columns}}` | Compile (hook / test usage) | Column metadata from the transformation (excludes column tests); supports `{{#model.columns}}` sections |
| `{{model.tested_column}}` | Compile (column test usage) | Column metadata for the column the test is assigned to; absent for transformation-level tests. Fields: `name`, `description`, `data_type`, `labels` |
| `{{model.handler}}` | Compile (hook / test usage) → Runtime | At usage compile, becomes `{{<model_name>}}`; at runtime, resolves to table ID. Works in template bodies |
| `{{vars__<key>}}` / `{{var__<key>}}` | Runtime | CLI variables (e.g. `{{var__report_year}}`) |
| `{{<model_name>}}` | Runtime | Table ID for a model (from catalog or `--mappings`) |
| `{{session_id}}` | Runtime | UUID with unique session id of this execution |

#### Hook model context

When an operation runs as a hook (Init / Pre / Post on a transformation), a `{{model.*}}` namespace is available. Model context comes from the **transformation that defines the hook** (`Transformation.model` and `Transformation.columns`), not from the operation itself.

**Compile phases for hooks.** Hook SQL is built in two compile steps:

1. **Template / operation inline** — templates and operation code are merged; `{{props__*}}` from config is resolved where available. All `{{model.*}}` tags are **preserved** through this step (including inside nested template chains), so they survive operation template wrapping.
2. **Hook usage compile** — each hook reference gets its own SQL in `OperationUsage.sql_code`; `{{props__*}}` (config + hook prop overrides) and all `{{model.*}}` fields are resolved here. `{{model.handler}}` becomes `{{<model_name>}}` (e.g. `{{dummy_model}}`).

At **runtime**, only `{{vars__*}}`, `{{session_id}}`, and model table refs (`{{<model_name>}}`, including the tag emitted by `{{model.handler}}`) are resolved.

**Templates.** Hook operations may use templates (including nested template chains). Put `{{model.handler}}`, `{{model.name}}`, `{{#model.columns}}`, etc. in the template body or operation code — model variables work in both. Example with a template:

```sql
-- template body:     CREATE TABLE tmp AS SELECT * FROM {{model.handler}} WHERE {{props__code}}
-- operation code:    SELECT 1
-- after hook compile: CREATE TABLE tmp AS SELECT * FROM {{dummy_model}} WHERE SELECT 1
-- after runtime:      CREATE TABLE tmp AS SELECT * FROM dataset.dummy_model WHERE SELECT 1
```

Handler indirection example:

```sql
-- in operation spec:  SELECT * FROM {{model.handler}}
-- after hook compile: SELECT * FROM {{dummy_model}}
-- after runtime:       SELECT * FROM dataset.dummy_model
```

Column iteration at compile time:

```sql
-- in operation spec:  {{#model.columns}}`{{name}}` {{data_type}},{{/model.columns}}
-- after hook compile: `id` INT64, `name` STRING,
```

`{{model.*}}` is only defined for hook runs and transformation/column test runs. Operations that use model variables should not be run via standalone `apply`.

#### Test model context

When a test runs from a transformation (model-level **Tests** section or column **Tests** section), the same `{{model.*}}` namespace is available at compile time. Each test reference gets its own SQL in `TestUsage.sql_code`, built in two compile steps (same as hooks):

1. **Global test compile** — `{{props__*}}` from config and test default props are resolved; `{{model.*}}` tags are preserved when the test is referenced from a transformation.
2. **Test usage compile** — props (config + usage overrides + test default props) and all `{{model.*}}` fields are resolved per reference.

For tests assigned on a **column**, `{{model.tested_column}}` is set to that column's metadata (`name`, `description`, `data_type`, `labels`). It is absent for transformation-level tests.

Example column test SQL:

```sql
-- in test spec:  SELECT COUNT(*) FROM {{model.handler}} WHERE {{model.tested_column.name}} IS NULL
-- after compile:  SELECT COUNT(*) FROM {{dummy_model}} WHERE amount IS NULL
-- after runtime:  SELECT COUNT(*) FROM dataset.dummy_model WHERE amount IS NULL
```

`{{model.*}}` is only defined for test runs from a transformation. Tests invoked via standalone `test --names my_test` use the global test SQL without model context.

Variable syntax follows the [mustache](https://lib.rs/crates/mustache) crate: `{{name}}`. Use `{{var__name}}` (or `{{vars__name}}`) for CLI variables and `{{props__name}}` for config/template props.
</details>

## Spec format

Specs are Markdown files with a fixed heading structure. Each file describes one entity; the `## Type` section declares its kind (`model`, `transformation`, `template`, `test`, `operation`, or `config`).

See [specs/README.md](https://github.com/Dataspecs/specs/blob/main/README.md) for the full format reference, [specs/data-specs/](https://github.com/Dataspecs/specs/tree/main/data-specs/) for minimal examples, and [specs/examples/eth/](https://github.com/Dataspecs/specs/tree/main/examples/eth/) for a realistic dependency graph.

---

## Scaffolding CLI reference

```bash
dataspec new <name> [--path DIR]
```

| Flag | Description |
|------|-------------|
| `--path` | Directory to create the project in (default: current directory) |

---

## Library API

This crate is a library used by generated projects. Main entry points:

| Function | Used in | Purpose |
|----------|---------|---------|
| `spec_builder(specs_dir, output_path)` | `build.rs` | Parse specs, generate `data.rs` |
| `spec_handler(catalog)` | `main.rs` | Runtime CLI (`transform`, `apply`, `test`, `list`) |

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
- Partition/Clusters parsing
---

## License

Apache-2.0
