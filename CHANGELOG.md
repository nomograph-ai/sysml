# Changelog

All notable changes to nomograph-sysml are documented here.

## [Unreleased]

## [0.1.0] — 2026-03-09

Initial public release under Nomograph Labs.

### Added

**CLI commands** — single binary, dual-mode (CLI + MCP server):

- `parse` — parse SysML v2 source files with tree-sitter-sysml
- `validate` — syntax validation with structured error output
- `index` — build a persistent knowledge graph from a SysML v2 model directory
- `search` — 9-signal hybrid discovery index (keyword, vector, graph signals)
- `inspect` — exact-name element lookup with full relationship context
- `trace` — multi-hop relationship traversal from any element
- `check` — structural and metamodel completeness checks
- `query` — predicate-based relationship search
- `render` — template-based reports (traceability matrix, requirements table, completeness)
- `stat` — model health dashboard with shields.io-style SVG badge output
- `plan` — decompose a question into executable CLI commands
- `diff` — compare two knowledge graph indexes
- `scaffold` — generate SysML v2 scaffold text (8 element kinds)
- `skill` — generate an agent skill file (~300 tokens)

**MCP server** (feature-gated: `--features mcp`):

- 15 MCP tools exposing the full knowledge graph over the Model Context Protocol
- 4 MCP prompts for common MBSE reasoning patterns
- 3 server configuration flags

**Knowledge graph**:

- 27 SysML v2 relationship types indexed
- 9-signal hybrid scorer (keyword + vector + graph)
- Vector search via fastembed all-MiniLM-L6-v2 (384-dim, HNSW index)
- RFLP layer support (Requirement, Functional, Logical, Physical)

**CI/CD integration**:

- `ci/nomograph-sysml.gitlab-ci.yml` — reusable validation gate (syntax + completeness)
- `ci/nomograph-mr-diff.gitlab-ci.yml` — model diff on merge requests

**Architecture**: three-crate Rust workspace — `nomograph-core` (generic traits),
`sysml-core` (SysML v2 domain), `sysml-cli` (CLI + MCP transport).

### Dependencies

- `tree-sitter-sysml` 0.1.0 — SysML v2 grammar (99.6% external coverage)
- `rmcp` 0.1 — Model Context Protocol SDK
- `fastembed` 5.12 — local vector embeddings

[0.1.0]: https://gitlab.com/nomograph/sysml/-/releases/v0.1.0
