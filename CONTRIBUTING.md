# Contributing to nomograph-sysml

## Grammar Sync: tree-sitter-sysml

This project depends on [tree-sitter-sysml](https://gitlab.com/nomograph/tree-sitter-sysml) as a local path dependency. The grammar parses SysML v2 into AST nodes; the **walker** (`crates/sysml-core/src/walker.rs`) decides which AST nodes to extract elements and relationships from.

The grammar and the walker can drift. When tree-sitter-sysml adds or changes node types, the walker may silently ignore them. To catch this, we maintain a suite of **coverage tests** that fail with descriptive messages when sync is needed.

## Coverage Tests

All coverage tests live in `crates/sysml-core/src/graph.rs` under `mod coverage_tests`. They run as part of `cargo test --workspace` — no extra tooling needed.

### What the tests catch

| Scenario | Test | Failure message |
|----------|------|-----------------|
| New grammar element type appears in corpus | `test_parsed_element_kinds_in_vocabulary` | `Eve corpus contains element kind 'X' not in ELEMENT_KIND_NAMES` |
| New grammar relationship type appears in corpus | `test_parsed_relationship_kinds_in_vocabulary` | `Eve corpus contains relationship kind 'X' not in RELATIONSHIP_KIND_NAMES` |
| New `RelationshipKind` enum variant added without vocabulary entry | `test_relationship_kind_names_covers_all_variants` | `RelationshipKind::X missing from RELATIONSHIP_KIND_NAMES` |
| Deleted `RelationshipKind` variant still in vocabulary | `test_relationship_kind_names_no_stale_entries` | `RELATIONSHIP_KIND_NAMES contains 'X' but no variant produces it` |
| New `RelationshipKind` variant with no dispatch entry | `test_dispatch_covers_all_non_synthetic_variants` | `RelationshipKind::X has no entry in RELATIONSHIP_DISPATCH` |
| New element kind added to vocabulary without RFLP layer decision | `test_classify_layer_covers_all_element_kinds` | `Element kind 'X' returns None from classify_layer but is not in INTENTIONAL_NO_LAYER` |
| Kind moved to a layer but still in no-layer allowlist | `test_intentional_no_layer_entries_are_valid` | `INTENTIONAL_NO_LAYER contains 'X' but classify_layer returns Some` |
| `STRUCTURAL_RELATIONSHIP_KINDS` references invalid kind | `test_structural_kinds_are_valid_relationship_kinds` | `STRUCTURAL_RELATIONSHIP_KINDS contains 'X' which is not in RELATIONSHIP_KIND_NAMES` |
| `plan.rs` hardcoded relationship strings drift from enum | `test_plan_rel_strings_are_valid_relationship_kinds` | `plan.rs hardcodes 'X' which is not in RELATIONSHIP_KIND_NAMES` |

### What the tests do NOT catch

These require manual review when making changes:

1. **Wrong extraction logic via wildcard arm**: When a new `RelationshipKind` variant is added, `extract_relationship` in `walker.rs` has a `_ =>` catch-all that uses generic `extract_reference_text`. This may produce correct results for simple cases but miss source/target for complex node structures (e.g., `message_statement` with `from`/`to` feature chains). Always add an explicit match arm for new variants.

2. **Metamodel substring checks**: `metamodel.rs` functions like `is_logical_kind` and `is_physical_kind` use `contains()` substring matching. A new element kind that doesn't contain the expected substrings ("part", "port", "requirement", etc.) will silently pass metamodel checks.

3. **MCP/CLI feature parity**: New CLI flags don't automatically get MCP equivalents. Intentional omissions: `plan`, `skill`, `--format` are CLI-only. The `search --layer` and `trace --trace-format` CLI flags have no MCP equivalent yet.

## Procedures

### When tree-sitter-sysml adds a new node type

1. Run `cargo test --workspace` — coverage tests will tell you exactly what's missing
2. If a new **element kind** appears:
   - Add it to `ELEMENT_KIND_NAMES` in `vocabulary.rs`
   - Decide its RFLP layer in `classify_layer()`, or add to `INTENTIONAL_NO_LAYER` in the coverage test
   - Add vocabulary aliases to `ELEMENT_KIND_MAP` if the kind has common synonyms
3. If a new **relationship-bearing statement** appears:
   - Add a `RelationshipKind` variant (and `Display` arm, and entry in `ALL`)
   - Add a dispatch entry in `RELATIONSHIP_DISPATCH`
   - Add an extraction arm in `extract_relationship` (don't rely on the `_` fallback)
   - Add the kind string to `RELATIONSHIP_KIND_NAMES` in `vocabulary.rs`
4. Run `cargo test --workspace && cargo clippy --workspace` to verify

### When adding a new RelationshipKind variant

1. Add the variant to the `enum RelationshipKind`
2. Add a `Display` arm
3. Add it to `RelationshipKind::ALL`
4. Add a dispatch entry in `RELATIONSHIP_DISPATCH` (unless synthetic like `TypedBy`/`Member`)
5. Add an explicit `extract_relationship` match arm
6. Add the display string to `RELATIONSHIP_KIND_NAMES` in `vocabulary.rs`
7. Run tests — all 6 coverage tests for relationship kinds will verify the above

### When adding a new CLI flag

1. Add the clap derive field to the relevant `Args` struct in `main.rs`
2. If the flag affects domain logic, add it to the corresponding type in `nomograph-core/src/types.rs`
3. Consider whether the MCP equivalent needs the same parameter
4. Run the full test suite

## Test Fixtures

Coverage tests parse all fixture corpora and verify that every element kind and relationship kind found is known to the vocabulary.

| Corpus | Path | Files | Description |
|--------|------|-------|-------------|
| Eve Mining Frigate | `tests/fixtures/eve/` | 19 | LLM-generated EVE Online mining frigate model (Hugo Ormo) |
| Apollo 11 | `tests/fixtures/apollo-11/` | 28 | Airbus CoSMA framework -- 5 architectural layers (MPL-2.0) |

If either corpus is extended with new SysML v2 constructs, coverage tests will automatically flag any that the walker doesn't handle.
