---
status: active
project: nomograph-sysml
created: 2026-03-05
updated: 2026-03-07
depends_on: []
---

# PRD: nomograph-sysml — CLI-Native Knowledge Graph for SysML v2

## Vision

Build an open-source, CLI-native knowledge graph toolkit for digital engineering
languages — starting with SysML v2. Inspired by GitLab's Global Knowledge Graph
(GKG/Orbit) architecture: tree-sitter parsing → property graph indexing → agentic
graph traversal → token-efficient retrieval.

**Core thesis**: LLMs are native CLI speakers. A `nomograph-sysml trace` command
that costs ~50 tokens to invoke and returns structured JSON outperforms an MCP
tool that burns ~15,000 tokens loading schemas before work begins. The knowledge
graph is the essential component — not the transport protocol.

**Dual mode**: Single binary serves as both CLI tool and MCP server (`--mcp` flag).
Domain logic lives in `sysml-core`; CLI and MCP are thin transport wrappers.

**Strategic intent**: Demonstrate a valuable approach to adding MBSE language
support to GitLab's GKG portfolio. Substantiate claims through benchmarks.

## Evidence Base

### Internal Benchmarks

| Finding | Source | Implication |
|---------|--------|-------------|
| **CLI search S=0.900 beats MCP S=0.847** | overnight benchmarks | CLI-first approach validated |
| ~~Graph tools hurt accuracy by 8pp~~ **Graph tools beat search by +3.5pp overall** | round-2 benchmarks | Relationship fix resolved the deficit |
| Graph tools dominate one-hop (+6.7pp) and global (+15.6pp) | round-2 benchmarks | Graph traversal is the right tool for structured queries |
| Multi-hop at parity (-1.0pp, was -10.9pp deficit) | round-2 benchmarks | Relationship completeness fix closed the gap |
| Knowledge graph is sole source of benefit (RQ2) | analysis-report.md | Graph tools are core commands |
| MCP tools hurt gpt-4o (-23.5pp) and gpt-4o-mini (-10.4pp) | analysis-report.md | Simpler interface reduces harm |
| 9 of 16 tasks are ceiling tasks (both score 1.0) | analysis-report.md | Need harder tasks |
| Token reduction implemented | session | check --detail, trace --max-results, query --compact |
| **Query/trace excluded 58% of edges** | bug analysis | Member (765) + Import (83) hardcoded skip; fixed with conditional filters |
| **Assert satisfy mis-extraction** | bug analysis | Nested `assert satisfy` fell through to generic Assert arm |
| Grammar gap: 6 missing dispatch entries | walker analysis | succession, first, stream, specialization, redefines, message statements |
| Grammar gap: stale `exhibit_statement` key | walker analysis | Grammar has `exhibit_usage`, not `exhibit_statement` |

### External Prior Art

| Finding | Source | Implication |
|---------|--------|-------------|
| Neo4j GraphRAG achieves 93% on 100 Q&A (BEV model) | Quast 2026 | Our 90% competitive; no graph DB needed |
| Multi-hop drops to 50-64% on weaker models | Quast 2026 | Query decomposition needed |
| Tri-layered KG catches hallucinations | Qualis 2025 | Metamodel conformance rules |
| py-capellambse ships GitLab CI templates | DB InfraGO | CI/CD integration proven |
| No open-source CLI combines discovery+authoring+validation for SysML v2 | survey | First-mover |
| GKG uses 6 MCP tools | GKG internal | Our commands map directly |

## Architecture

### Crate Topology

```
nomograph-sysml/
├── crates/
│   ├── nomograph-core/   ← generic traits: Graph, Index, Parser, Scorer, Vocabulary
│   ├── sysml-core/       ← SysML v2 domain: vocabulary, relationships, validation, render
│   └── sysml-cli/        ← clap CLI binary + MCP server (feature-gated)
├── tests/fixtures/       ← SysML v2 test files (Eve model)
└── memory/               ← session handoff and observations
```

### Dependency Flow

```
tree-sitter-sysml (external)
       ↓
nomograph-core (generic traits)
       ↓
sysml-core (SysML v2 domain)
       ↓
sysml-cli (CLI mode: clap | MCP mode: rmcp, feature-gated)
```

### Dual Mode Architecture

The `sysml-cli` crate serves both CLI and MCP modes from a single binary:

- **CLI mode** (default): clap-derive command parsing, JSON to stdout, logs to stderr
- **MCP mode** (`--mcp` flag): rmcp server over stdio, JSON-lines framing

MCP dependencies are feature-gated (`--features mcp`) to keep the default binary lean:
- Default binary: ~4.2 MB
- With MCP: ~6.5 MB

MCP tools delegate directly to sysml-core functions — no duplicated domain logic.

### MCP Tools (10)

| MCP Tool | CLI Equivalent | Description |
|----------|---------------|-------------|
| `sysml_index` | `index` | Build knowledge graph from files |
| `sysml_search` | `search` | Search by name, kind, or text |
| `sysml_trace` | `trace` | Traverse relationship chains |
| `sysml_check` | `check` | Run structural + metamodel checks |
| `sysml_query` | `query` | Predicate-based relationship search |
| `sysml_render` | `render` | Template-based report generation |
| `sysml_stat` | `stat` | Model health dashboard |
| `sysml_inspect` | `inspect` | Exact-name element lookup with full context |
| `read_file` | N/A | Read file contents with offset/limit |
| `sysml_validate` | `validate` | Validate SysML v2 syntax |

### nomograph-core Trait System

Generic traits that any digital engineering language can implement:

```rust
pub trait Element: Send + Sync {
    fn qualified_name(&self) -> &str;
    fn kind(&self) -> &str;
    fn file_path(&self) -> &Path;
    fn span(&self) -> Span;
    fn metadata(&self) -> &dyn Any;
}

pub trait Relationship: Send + Sync {
    fn source(&self) -> &str;
    fn target(&self) -> &str;
    fn kind(&self) -> &str;
    fn file_path(&self) -> &Path;
    fn span(&self) -> Span;
}

pub trait Parser: Send + Sync {
    type Elem: Element;
    type Rel: Relationship;
    type Error: std::error::Error;
    fn parse(&self, source: &str, path: &Path) -> Result<ParseResult<Self::Elem, Self::Rel>, Self::Error>;
    fn validate(&self, source: &str) -> Vec<Diagnostic>;
}

pub trait KnowledgeGraph: Send + Sync {
    type Elem: Element;
    type Rel: Relationship;
    fn index(&mut self, results: Vec<ParseResult<Self::Elem, Self::Rel>>) -> Result<(), IndexError>;
    fn search(&self, query: &str, level: DetailLevel, limit: usize) -> Vec<SearchResult>;
    fn trace(&self, element: &str, opts: TraceOptions) -> TraceResult;
    fn check(&self, check_type: CheckType) -> Vec<Finding>;
    fn query(&self, predicate: Predicate) -> Vec<Triple>;
    fn elements(&self) -> &[Self::Elem];
    fn relationships(&self) -> &[Self::Rel];
}

pub trait Vocabulary: Send + Sync {
    fn expand_kind(&self, kind: &str) -> Vec<&str>;
    fn normalize_kind(&self, kind: &str) -> &str;
    fn relationship_kinds(&self) -> &[&str];
    fn element_kinds(&self) -> &[&str];
}

pub trait Scorer: Send + Sync {
    fn score(&self, query: &str, candidates: &[ScoringCandidate]) -> Vec<ScoredResult>;
    fn signals(&self) -> &[&str];
}
```

---

## CLI Design

### Binary Name

`nomograph-sysml` — installed via `cargo install nomograph-sysml`.

### Command Structure

```
nomograph-sysml [--mcp] <COMMAND> [OPTIONS]

Commands:
  parse       Parse SysML v2 source files
  validate    Check SysML v2 source for errors
  index       Build a knowledge graph from SysML v2 files
  search      Search the knowledge graph
  inspect     Exact-name element lookup with full context
  trace       Traverse relationships from an element
  check       Run structural completeness + metamodel checks
  query       Query relationships by predicate
  render      Render model reports from templates
  stat        Show model health dashboard
  plan        Decompose a question into executable CLI commands
  diff        Compare two knowledge graph indexes
  scaffold    Generate SysML v2 scaffold text
  skill       Generate agent skill files and harness scaffold

Global Options:
  --mcp               Start as MCP server over stdio (feature-gated)
  --format <FORMAT>    Output format: json (default), pretty, compact
  --quiet              Suppress non-essential output
  --verbose            Enable debug logging to stderr
  --version            Print version
  --help               Print help
```

### Command Specifications

#### `parse`

```
nomograph-sysml parse <FILES...> [--level <LEVEL>]

Parse one or more SysML v2 files and output the AST.

Options:
  --level <LEVEL>   Detail level: l0 (names), l1 (structure), l2 (full AST) [default: l1]
```

#### `validate`

```
nomograph-sysml validate <FILES...> [--strict]

Validate SysML v2 files and report diagnostics.

Options:
  --strict    Treat warnings as errors
```

#### `index`

```
nomograph-sysml index <PATHS...> [--output <PATH>] [--vectors]

Build a knowledge graph index from SysML v2 files.

Options:
  --output <PATH>   Write index to file [default: .nomograph/index.json]
  --vectors         Enable vector embeddings (not yet implemented)
```

#### `search`

```
nomograph-sysml search <QUERY> [--index <PATH>] [--level <LEVEL>] [--limit <N>] [--kind <KIND>]

Search the knowledge graph for elements matching a query.

Options:
  --index <PATH>   Path to index file [default: .nomograph/index.json]
  --level <LEVEL>  Detail level: l0, l1, l2 [default: l1]
  --limit <N>      Max results [default: 10]
  --kind <KIND>    Filter by element kind
```

#### `inspect`

```
nomograph-sysml inspect <NAME> [--index <PATH>]

Exact-name element lookup returning full context: kind, layer, members,
inbound/outbound relationships, span, and file path.
```

#### `trace`

```
nomograph-sysml trace <ELEMENT> [--index <PATH>] [--hops <N>] [--direction <DIR>]
                      [--types <TYPES...>] [--trace-format <FMT>] [--max-results <N>]
                      [--include-structural]

Traverse the relationship graph from a starting element.

Options:
  --hops <N>              Maximum traversal depth [default: 3]
  --direction <DIR>       forward, backward, both [default: both]
  --types <TYPES...>      Filter by relationship type
  --trace-format <FMT>    Output shape: chain, tree, flat [default: chain]
  --max-results <N>       Limit number of hops in output (token reduction)
  --include-structural    Include Member/Import edges (excluded by default)
```

#### `check`

```
nomograph-sysml check <CHECK_TYPE...> [--index <PATH>] [--scope <SCOPE>]
                      [--fail-on-findings] [--detail]

Run structural completeness + metamodel checks.

Check types:
  orphan-requirements, unverified-requirements, missing-verification,
  unconnected-ports, dangling-references, metamodel-conformance, all

Options:
  --scope <SCOPE>      Limit to elements in a namespace
  --fail-on-findings   Exit code 1 if findings exist
  --detail             Show full findings instead of summary counts
```

#### `query`

```
nomograph-sysml query [--index <PATH>] [--source-kind <KIND>] [--source-name <NAME>]
                      [--rel <KIND>] [--target-kind <KIND>] [--target-name <NAME>]
                      [--source-layer <LAYER>] [--target-layer <LAYER>]
                      [--exclude-rel <KINDS>] [--limit <N>] [--compact]

Query relationships by predicate.

Options:
  --compact              One-line output: source -> rel -> target (token reduction)
  --source-layer <LAYER> Filter source by RFLP layer
  --target-layer <LAYER> Filter target by RFLP layer
  --exclude-rel <KINDS>  Comma-separated relationship kinds to exclude
```

#### `render`

```
nomograph-sysml render --template <NAME> [--index <PATH>] [--render-format <FMT>]
                       [--custom <PATH>]

Render model reports from built-in or custom templates.

Built-in templates:
  traceability-matrix     Requirements with satisfy/verify status and coverage
  requirements-table      Sorted requirement listing with documentation
  completeness-report     Model health dashboard with all check summaries

Options:
  --render-format <FMT>  markdown (default), html, csv
  --custom <PATH>        Path to custom Handlebars (.hbs) template
```

#### `stat`

```
nomograph-sysml stat [--index <PATH>] [--badge]

Show model health dashboard: element/relationship counts, kind distribution, file breakdown.

Options:
  --badge    Output SVG health badge instead of JSON
```

#### `diff`

```
nomograph-sysml diff <BASE> <HEAD> [--compact]

Compare two knowledge graph indexes and report changes.

Arguments:
  BASE    Path to base index file (before changes)
  HEAD    Path to head index file (after changes)

Options:
  --compact    One-line-per-change summary instead of full JSON
```

#### `scaffold`

```
nomograph-sysml scaffold <KIND> <NAME> [--raw]

Generate SysML v2 scaffold text for a given element kind.

Arguments:
  KIND    Element kind: requirement, verification, part, package, use-case, action, state, interface
  NAME    Name for the generated element

Options:
  --raw    Output raw SysML v2 text instead of JSON wrapper
```

#### `skill`

```
nomograph-sysml skill [--scan] [--output <PATH>] [--harness]

Generate agent skill file or harness scaffold.

Modes:
  (bare)       Output markdown skill file to stdout (~300 tokens)
  --scan       Detect other nomograph-* binaries on $PATH, composite skill file
  --harness    Generate .nomograph/ scaffold with skill file, config, scripts
```

### Conventions

- **JSON output**: All commands output JSON to stdout, diagnostics to stderr. Compact by default; `--format pretty` adds indentation.
- **Composability**: Commands pipe via `jq` (e.g., `check orphan-requirements | jq -r '.[].element' | xargs -I{} trace {} --types satisfy`).
- **Exit codes**: 0=success, 1=validation error, 2=IO error, 3=index not found. Empty results return 0 with `[]`.
- **Index persistence**: `.nomograph/index.json` auto-discovered in CWD or parents. Override with `--index`.

---

## Implementation Phases

### Phase 1: Workspace Scaffold + nomograph-core Traits ✅

Cargo workspace with 3 crates, all traits and shared types, clap skeleton.

### Phase 2: Parser Integration ✅

`parse` and `validate` work on real .sysml files using tree-sitter-sysml.
AST walker extracts elements and relationships. L0/L1/L2 detail levels.

### Phase 3: Knowledge Graph Index ✅

`index` builds queryable knowledge graph. `search` returns scored results.
Cross-file resolution, multi-signal scorer, index serialization.

### Phase 4: Graph Tools — trace, check, query ✅

BFS traversal with hop/direction/type filters. 5 structural completeness checks.
Predicate-based relationship search with glob matching.

### Phase 5: Skill File + Stat + Token Reduction ✅

`skill` command outputs ~300-token markdown. `stat` command for model health dashboard.
Token reduction: `check --detail`, `trace --max-results`, `query --compact`.

### Phase 6: Vector Search (Deferred)

Semantic search via fastembed. Not needed for v1 — structural signals achieve S=0.900.

### Phase 7: Benchmark Integration ✅

CLI benchmark condition integrated. CLI search S=0.900 beats MCP S=0.847.
162 tasks across 11 YAML files. Overnight cost: $755.72.

### Phase 8: Render + Metamodel Checks ✅

**Render command**: 3 built-in templates (traceability-matrix, requirements-table,
completeness-report) × 3 formats (markdown, html, csv) + custom Handlebars support.
Template engine: handlebars-rust 6.4.0.

**Metamodel conformance**: 5 semantic validation rules:
- satisfy targets must be requirement elements
- verify targets must be requirement elements
- allocate source must be logical, target physical
- ports must have TypedBy relationships
- binding connectors must connect compatible types

Added to `check metamodel-conformance` and included in `check all`.

**MCP server mode**: `nomograph-sysml --mcp` starts rmcp server over stdio.
Feature-gated behind `--features mcp`. 10 tools (7 original + read_file + sysml_validate +
sysml_inspect), all delegate to sysml-core.

**RFLP layer typing**: Each element classified as Requirements/Functional/Logical/Physical
based on SysML kind. Exposed in search detail, stat breakdown, and `--layer`/`--source-layer`/
`--target-layer` filters. Physical layer deferred (needs package-path context, not just kind).

### Phase 9: Plan / Query Decomposition ✅

**Goal**: `plan` command decomposes complex questions into executable sub-queries.

7 question types: Relationship, Reverse, Completeness, Comparison, Impact, Discovery, Global.
Heuristic entity extraction (uppercase words, fallback to long tokens). Each type maps to
a specific sequence of search/trace/query/check/render commands.

`--execute` flag runs the plan via subprocess, aggregating results into a single JSON output.

14 tests covering classification, entity extraction, and decomposition patterns.

### Phase 9.5: Graph Data Completeness + Inspect ✅

**Goal**: Fix graph tool data completeness (58% of edges invisible) and add
speculative enhancements for multi-hop benchmark performance.

**Bug fixes**:
- query and trace hardcoded `skip_kinds = ["import", "member"]`, filtering 848/1454
  relationships. Removed from query; made conditional in trace (`--include-structural`).
- `assert satisfy` nested inside `assert_statement` fell through to generic `_` arm,
  recording `Assert` instead of `Satisfy`. Fixed with explicit `Assert` arm checking
  for nested satisfy/verify children.

**Enhancements**:
- `inspect` command + `sysml_inspect` MCP tool: exact-name element lookup returning
  kind, layer, members[], relationships_in/out, span, file_path. 13th CLI command, 10th MCP tool.
- Trace hops enriched: `source_kind`, `target_kind`, `source_layer`, `target_layer`
  populated via `resolve_element()` during BFS.
- `stat` includes `relationship_breakdown` by kind.
- `query --exclude-rel` filter: comma-separated exclusion list. Added to `Predicate` struct.
- MCP query: `source_layer`, `target_layer`, `exclude_rel` parameters added.

**Grammar gap analysis** (identified here, fixed in Phase 10):
- `succession_statement`, `first_statement`, `stream_statement`, `specialization_statement`,
  `redefines_statement`, `message_statement` missing from walker RELATIONSHIP_DISPATCH.
- `"exhibit_statement"` key in dispatch table is stale (grammar has `exhibit_usage`).
- `flow_usage` hidden gap: `flow of X from A to B` parsed as `flow_usage`, not `flow_statement`.

81 tests passing (up from 72). Zero clippy warnings.

### Phase 10: Scaffold + Diff ✅

**Goal**: LLM authoring support (scaffold) and model comparison (diff).

**Deliverables**:
- [x] 10.1: `scaffold` generates valid SysML v2 text for 8 element kinds
  - requirement, verification, part, package, use-case, action, state, interface
  - `--raw` flag for plain SysML text output (vs JSON wrapper)
  - Parseability validated: all 8 kinds parse without tree-sitter errors
- [x] 10.2: `diff` compares two indexes
  - Added/removed/modified elements and relationships
  - `--compact` flag for one-line-per-change summary
  - 7 tests covering all change types and compact format
- [x] 10.3: Grammar gap fixes (8 walker dispatch entries)
  - first/message/succession/redefines/specialization/stream/flow_usage/exhibit_usage
  - 2 new relationship kinds: Message, Stream
  - 14 element kinds added to vocabulary
  - 9 coverage tests guard against grammar drift
  - 5 extraction tests validate new dispatch entries
  - Relationships: 1454 → 1515 (+61). Tests: 81 → 112.

15 CLI commands. 10 MCP tools.

### Phase 11: CI/CD Pipeline Integration ✅

**Goal**: GitLab CI templates and pipeline-native model validation.

**Deliverables**:
- [x] 11.1: `.gitlab-ci.yml` template: model validation gate
  - `sysml-validate` job: syntax checks all `.sysml` files, fails on errors
  - `sysml-check` job: indexes, runs completeness checks, generates badge artifact
  - Configurable via CI variables: model dir, checks, fail-on-findings
- [x] 11.2: MR diff template: base branch vs head branch index comparison
  - Runs on MR pipelines when `.sysml` files change
  - Produces `model-diff.json` artifact with compact/full modes
- [x] 11.3: Model health badge (SVG) via `stat --badge`
  - Shields.io-style badge: "model health | N% (XE / YR)"
  - Color-coded: green (100%), yellow (>=80%), orange (>=50%), red (<50%)
  - `badge.rs` module with `compute_badge_data()` + `render_svg()`, 4 tests
- [x] 11.4: Documentation: CI/CD integration guide in README.md

15 CLI commands. 10 MCP tools. 116 tests.

---

## Technical Decisions

### Dependencies

```toml
[workspace.dependencies]
tree-sitter = "0.24"
tree-sitter-sysml = { path = "../../tree-sitter-sysml" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4", features = ["derive"] }
thiserror = "2.0"
anyhow = "1.0"
handlebars = "6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# MCP mode (feature-gated)
rmcp = { version = "0.1", features = ["server", "transport-io"] }
tokio = { version = "1.43", features = ["full"] }
schemars = "0.8"

# Optional (Phase 6, deferred)
# fastembed = { version = "5.11", optional = true }
# hnsw_rs = { version = "0.3", optional = true }
```

### Relationship to open-mcp-sysml

| Aspect | open-mcp-sysml (MCP) | nomograph-sysml (CLI) |
|--------|----------------------|----------------------|
| Transport | JSON-RPC over stdio | Shell commands + JSON stdout |
| Discovery | MCP tools/list (~15K tokens) | SKILL.md (~300 tokens) |
| State | In-memory server process | .nomograph/ directory (stateless) |
| Composability | All data through agent context | Unix pipes between commands |
| MCP mode | Standalone server | Feature-gated in same binary |
| Shared core | Duplicated domain logic | sysml-core is single source of truth |

---

## Interaction Domains

| Domain | Commands | Status | Prior Art |
|--------|----------|--------|-----------|
| **Discovery** | search, inspect, stat | ✅ Complete | Quast 2026 (GraphRAG, 93%) |
| **Retrieval** | trace, query, check | ✅ Complete | Quast 2026 (Cypher queries) |
| **Explanation** | render | ✅ Complete | py-capellambse (Jinja2) |
| **Verification** | check (metamodel) | ✅ Complete | Qualis 2025 (metamodel) |
| **Orchestration** | plan | ✅ Complete | Quast 2026 (Supervisor Agent) |
| **Authoring** | scaffold | ✅ Complete | Qualis 2025, Ingrid (deltas) |
| **Comparison** | diff | ✅ Complete | Li 2025, capella-diff-tools |
| **Integration** | CI templates, badge | ✅ Complete | py-capellambse (GitLab CI) |

---

## GKG Alignment

| GKG Tool | nomograph-sysml Equivalent | Notes |
|----------|---------------------------|-------|
| `find_nodes` | `search` | Entity discovery by type/filter |
| `traverse_relationships` | `trace` | N-hop graph traversal |
| `find_paths` | `trace --direction both` + `plan` | Path-finding |
| `aggregate_nodes` | `stat`, `check`, `render` | OLAP-style aggregations |
| Schema introspection | `skill` | Tool/schema description |

### Novel Contributions

1. CLI-first discovery matches GraphRAG (90% vs 93%) with zero infrastructure
2. Token-efficient tool descriptions: 300 tokens vs 15K MCP schema
3. Summary-by-default output reduces graph tool context bloat
4. RFLP layer typing maps to GKG domain concept
5. Model diff in MR pipelines extends GKG code-change indexing to models

---

## Testable Hypotheses

| Hypothesis | Variable | Without | With | Measure |
|------------|----------|---------|------|---------|
| ~~Token reduction closes graph tool gap~~ | --detail/--max-results/--compact | cli_graph S=0.819 | **cli_graph +3.5pp over search** | ✅ Validated: relationship fix was the key intervention |
| RFLP layer typing improves cross-layer queries | layer field on SysmlElement | flat kinds | R/F/L/P tagged | S score on multi-hop questions |
| ~~Plan command closes multi-hop gap~~ | plan decomposition | cli_search S=0.900 | **multi-hop at parity (-1.0pp)** | ✅ Validated: parity achieved via relationship completeness, not plan |
| Render reduces synthesis burden | render pre-generates narratives | raw JSON | rendered markdown | Token count + S score on E1-E8 |
| Scaffold improves authoring accuracy | scaffold generates skeletons | LLM cold | scaffold + fill | Validity rate of generated SysML v2 |

---

## Success Criteria

### v0.1.0 (Phases 1-10) — Current

- [x] 15 commands + MCP mode produce correct output on Eve Mining Frigate model
- [x] 112 tests passing, 0 failing
- [x] `cargo clippy --workspace` clean
- [x] Skill file <400 tokens
- [x] CLI search S=0.900 on Eve discovery tasks
- [x] Render produces traceability-matrix, requirements-table, completeness-report
- [x] Metamodel conformance checks catch port typing issues
- [x] MCP server responds to initialize, tools/list, tools/call (10 tools)
- [x] RFLP layer typing: R/F/L classification, layer filters on search and query
- [x] Plan command decomposes 7 question types into CLI sequences
- [x] Plan `--execute` runs plans and aggregates results
- [x] Graph data completeness: all 15 relationship types queryable (was 58% hidden)
- [x] Inspect command: exact-name element lookup with full context
- [x] Enriched trace hops: source/target kind and layer in output
- [x] Binary size <50MB (4.2MB default, 6.5MB with MCP)
- [x] Coverage tests catch grammar/walker/vocabulary drift (9 sync tests)
- [x] `scaffold` generates parseable SysML v2 text (8 kinds, all parse cleanly)
- [x] `diff` accurately reports model changes between two indexes
- [x] 8 grammar gaps fixed, 1515 relationships extracted (was 1454)

### v0.2.0 (Phase 11) — Current

- [x] GitLab CI template validates model and gates merge
- [x] MR diff template: base branch vs head branch index comparison
- [x] `stat --badge` outputs SVG health badge
- [x] CI/CD integration documentation in README.md
- [ ] Benchmark: plan closes multi-hop gap vs Quast baseline
