use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, ServerHandler, ServiceExt};
use serde::Deserialize;
use tokio::sync::Mutex;

use sysml_core::core_traits::{KnowledgeGraph, Parser as NomographParser};
use sysml_core::core_types::{CheckType, DetailLevel, Direction, Predicate, TraceFormat, TraceOptions};
use sysml_core::element::RflpLayer;
use sysml_core::graph::{find_index, SysmlGraph};
use sysml_core::metamodel;
use sysml_core::render;
use sysml_core::SysmlParser;

#[derive(Clone)]
pub struct NomographServer {
    index_path: Arc<Mutex<Option<PathBuf>>>,
    graph: Arc<Mutex<Option<SysmlGraph>>>,
}

impl NomographServer {
    pub fn new() -> Self {
        Self {
            index_path: Arc::new(Mutex::new(None)),
            graph: Arc::new(Mutex::new(None)),
        }
    }

    async fn ensure_graph(&self) -> Result<(), String> {
        let mut graph_guard = self.graph.lock().await;
        if graph_guard.is_some() {
            return Ok(());
        }

        let idx_guard = self.index_path.lock().await;
        let path = match &*idx_guard {
            Some(p) => p.clone(),
            None => {
                let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
                find_index(&cwd).ok_or_else(|| {
                    "No index found. Use sysml_index to build one, or run `nomograph-sysml index` from your model directory.".to_string()
                })?
            }
        };

        let g = SysmlGraph::load(&path).map_err(|e| format!("Failed to load index: {}", e))?;
        *graph_guard = Some(g);
        Ok(())
    }

    fn err_json(msg: &str) -> String {
        serde_json::json!({"error": msg}).to_string()
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IndexRequest {
    #[schemars(description = "Directories or files to index")]
    pub paths: Vec<String>,
    #[schemars(description = "Output path for the index file (default: .nomograph/index.json)")]
    pub output: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchRequest {
    #[schemars(description = "Search query: element name, kind, or natural language")]
    pub query: String,
    #[schemars(description = "Maximum number of results (default: 10)")]
    pub limit: Option<usize>,
    #[schemars(description = "Filter by element kind (e.g., 'requirement_definition', 'part_definition')")]
    pub kind: Option<String>,
    #[schemars(description = "Detail level: l0, l1, l2 (default: l1)")]
    pub level: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TraceRequest {
    #[schemars(description = "Starting element name or qualified name")]
    pub element: String,
    #[schemars(description = "Maximum traversal depth (default: 3)")]
    pub hops: Option<u32>,
    #[schemars(description = "Traversal direction: forward, backward, both (default: both)")]
    pub direction: Option<String>,
    #[schemars(description = "Filter by relationship types (e.g., 'satisfy', 'verify')")]
    pub types: Option<Vec<String>>,
    #[schemars(description = "Maximum number of hops to return")]
    pub max_results: Option<usize>,
    #[schemars(description = "Include Member and Import edges (structural containment). Default: false, but auto-enabled when types includes 'member' or 'import'")]
    pub include_structural: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckRequest {
    #[schemars(description = "Check types: orphan-requirements, unverified-requirements, missing-verification, unconnected-ports, dangling-references, metamodel-conformance, or 'all' (default: all)")]
    pub checks: Option<Vec<String>>,
    #[schemars(description = "Filter findings by qualified name prefix")]
    pub scope: Option<String>,
    #[schemars(description = "Show full findings instead of summary counts (default: false)")]
    pub detail: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct QueryRequest {
    #[schemars(description = "Relationship kind to filter (e.g., 'satisfy', 'verify', 'typedby')")]
    pub rel: Option<String>,
    #[schemars(description = "Exclude relationship kinds (comma-separated, e.g., 'member,import')")]
    pub exclude_rel: Option<String>,
    #[schemars(description = "Filter by source element kind")]
    pub source_kind: Option<String>,
    #[schemars(description = "Filter by source element name")]
    pub source_name: Option<String>,
    #[schemars(description = "Filter by target element kind")]
    pub target_kind: Option<String>,
    #[schemars(description = "Filter by target element name")]
    pub target_name: Option<String>,
    #[schemars(description = "Maximum number of results (default: 50)")]
    pub limit: Option<usize>,
    #[schemars(description = "Filter by source RFLP layer: R, F, L, P")]
    pub source_layer: Option<String>,
    #[schemars(description = "Filter by target RFLP layer: R, F, L, P")]
    pub target_layer: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RenderRequest {
    #[schemars(description = "Built-in template: traceability-matrix, requirements-table, completeness-report")]
    pub template: String,
    #[schemars(description = "Output format: markdown, html, csv (default: markdown)")]
    pub render_format: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadFileRequest {
    #[schemars(description = "Path to the file to read")]
    pub path: String,
    #[schemars(description = "Starting line number (1-indexed, default: 1)")]
    pub offset: Option<usize>,
    #[schemars(description = "Maximum number of lines to return (default: all)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ValidateRequest {
    #[schemars(description = "Paths to SysML files or directories to validate")]
    pub paths: Vec<String>,
    #[schemars(description = "Treat warnings as errors (default: false)")]
    pub strict: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InspectRequest {
    #[schemars(description = "Element name or qualified name to inspect")]
    pub element: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StatRequest {}

fn parse_detail_level(s: Option<&str>) -> DetailLevel {
    match s {
        Some("l0") => DetailLevel::L0,
        Some("l2") => DetailLevel::L2,
        _ => DetailLevel::L1,
    }
}

fn collect_sysml_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_from_dir(path, &mut files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("sysml") {
            files.push(path.clone());
        }
    }
    files
}

fn collect_from_dir(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_from_dir(&path, files);
            } else if path.extension().and_then(|e| e.to_str()) == Some("sysml") {
                files.push(path);
            }
        }
    }
}

fn parse_check_type(s: &str) -> Option<CheckType> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "orphan_requirements" => Some(CheckType::OrphanRequirements),
        "unverified_requirements" => Some(CheckType::UnverifiedRequirements),
        "missing_verification" => Some(CheckType::MissingVerification),
        "unconnected_ports" => Some(CheckType::UnconnectedPorts),
        "dangling_references" => Some(CheckType::DanglingReferences),
        _ => None,
    }
}

#[tool(tool_box)]
impl NomographServer {
    #[tool(description = "Build a knowledge graph index from SysML v2 files. Run this before using other tools.")]
    async fn sysml_index(&self, #[tool(aggr)] req: IndexRequest) -> String {
        let paths: Vec<PathBuf> = req.paths.iter().map(PathBuf::from).collect();
        let sysml_files = collect_sysml_files(&paths);

        if sysml_files.is_empty() {
            return Self::err_json("No .sysml files found in provided paths");
        }

        let parser = SysmlParser::new();
        let mut results = Vec::new();
        for file_path in &sysml_files {
            let source = match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(e) => return Self::err_json(&format!("{}: {}", file_path.display(), e)),
            };
            match parser.parse(&source, file_path) {
                Ok(r) => results.push(r),
                Err(e) => return Self::err_json(&format!("{}: {}", file_path.display(), e)),
            }
        }

        let mut graph = SysmlGraph::new();
        if let Err(e) = graph.index(results) {
            return Self::err_json(&format!("Indexing failed: {}", e));
        }

        let index_path = match req.output {
            Some(ref p) => PathBuf::from(p),
            None => {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                cwd.join(".nomograph").join("index.json")
            }
        };

        if let Err(e) = graph.save(&index_path) {
            return Self::err_json(&format!("Saving index: {}", e));
        }

        let mut idx_guard = self.index_path.lock().await;
        *idx_guard = Some(index_path.clone());
        let mut graph_guard = self.graph.lock().await;
        let elem_count = graph.element_count();
        let rel_count = graph.relationship_count();
        let file_count = graph.file_count();
        *graph_guard = Some(graph);

        serde_json::json!({
            "files_indexed": file_count,
            "elements": elem_count,
            "relationships": rel_count,
            "index_path": index_path.to_string_lossy(),
        })
        .to_string()
    }

    #[tool(description = "Search the knowledge graph for elements by name, kind, or natural language query")]
    async fn sysml_search(&self, #[tool(aggr)] req: SearchRequest) -> String {
        if let Err(e) = self.ensure_graph().await {
            return Self::err_json(&e);
        }

        let graph_guard = self.graph.lock().await;
        let graph = graph_guard.as_ref().unwrap();
        let level = parse_detail_level(req.level.as_deref());
        let limit = req.limit.unwrap_or(10);
        let mut results = graph.search(&req.query, level, limit);

        if let Some(ref kind_filter) = req.kind {
            results.retain(|r| &r.kind == kind_filter);
        }

        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "qualified_name": r.qualified_name,
                    "kind": r.kind,
                    "file_path": r.file_path,
                    "score": r.score,
                    "detail": r.detail,
                })
            })
            .collect();

        serde_json::json!({
            "total_candidates": graph.element_count(),
            "results_returned": items.len(),
            "results": items,
        })
        .to_string()
    }

    #[tool(description = "Trace relationship chains from an element through the knowledge graph")]
    async fn sysml_trace(&self, #[tool(aggr)] req: TraceRequest) -> String {
        if let Err(e) = self.ensure_graph().await {
            return Self::err_json(&e);
        }

        let graph_guard = self.graph.lock().await;
        let graph = graph_guard.as_ref().unwrap();

        let direction = match req.direction.as_deref() {
            Some("forward") => Direction::Forward,
            Some("backward") => Direction::Backward,
            _ => Direction::Both,
        };

        let relationship_types = req.types.filter(|t| !t.is_empty());

        let include_structural = req.include_structural.unwrap_or(false)
            || relationship_types.as_ref().is_some_and(|types| {
                types.iter().any(|t| {
                    let tl = t.to_lowercase();
                    tl == "member" || tl == "import"
                })
            });

        let opts = TraceOptions {
            direction,
            max_hops: req.hops.unwrap_or(3),
            relationship_types,
            format: TraceFormat::Chain,
            include_structural,
        };

        let result = graph.trace(&req.element, opts);
        let total_hops = result.hops.len();
        let truncated = req.max_results.is_some_and(|max| total_hops > max);
        let hops: Vec<_> = if let Some(max) = req.max_results {
            result.hops.iter().take(max).collect()
        } else {
            result.hops.iter().collect()
        };

        serde_json::json!({
            "root": result.root,
            "total_hops": total_hops,
            "truncated": truncated,
            "hops": hops.iter().map(|h| {
                let mut hop = serde_json::json!({
                    "depth": h.depth,
                    "source": h.source,
                    "relationship": h.relationship,
                    "target": h.target,
                    "file_path": h.file_path,
                });
                if let Some(ref sk) = h.source_kind {
                    hop["source_kind"] = serde_json::json!(sk);
                }
                if let Some(ref tk) = h.target_kind {
                    hop["target_kind"] = serde_json::json!(tk);
                }
                if let Some(ref sl) = h.source_layer {
                    hop["source_layer"] = serde_json::json!(sl);
                }
                if let Some(ref tl) = h.target_layer {
                    hop["target_layer"] = serde_json::json!(tl);
                }
                hop
            }).collect::<Vec<_>>(),
        })
        .to_string()
    }

    #[tool(description = "Run structural completeness and metamodel conformance checks on the model")]
    async fn sysml_check(&self, #[tool(aggr)] req: CheckRequest) -> String {
        if let Err(e) = self.ensure_graph().await {
            return Self::err_json(&e);
        }

        let graph_guard = self.graph.lock().await;
        let graph = graph_guard.as_ref().unwrap();

        let check_names = req.checks.unwrap_or_else(|| vec!["all".to_string()]);
        let run_all = check_names.iter().any(|c| c == "all");
        let mut include_metamodel = run_all;

        let structural_checks: Vec<CheckType> = if run_all {
            vec![
                CheckType::OrphanRequirements,
                CheckType::UnverifiedRequirements,
                CheckType::MissingVerification,
                CheckType::UnconnectedPorts,
                CheckType::DanglingReferences,
            ]
        } else {
            let mut types = Vec::new();
            for name in &check_names {
                let normalized = name.to_lowercase().replace('-', "_");
                if normalized == "metamodel_conformance" || normalized == "metamodel" {
                    include_metamodel = true;
                } else if let Some(ct) = parse_check_type(name) {
                    types.push(ct);
                } else {
                    return Self::err_json(&format!(
                        "Unknown check type '{}'. Valid: orphan-requirements, unverified-requirements, missing-verification, unconnected-ports, dangling-references, metamodel-conformance, all",
                        name
                    ));
                }
            }
            types
        };

        let mut all_findings = Vec::new();
        let mut check_counts: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

        for ct in &structural_checks {
            let label = format!("{:?}", ct);
            let mut findings = graph.check(ct.clone());
            if let Some(ref scope) = req.scope {
                let scope_lower = scope.to_lowercase();
                findings.retain(|f| f.element.to_lowercase().starts_with(&scope_lower));
            }
            check_counts.insert(label, serde_json::json!(findings.len()));
            all_findings.extend(findings);
        }

        if include_metamodel {
            let mut findings = metamodel::run_metamodel_checks(graph);
            if let Some(ref scope) = req.scope {
                let scope_lower = scope.to_lowercase();
                findings.retain(|f| f.element.to_lowercase().starts_with(&scope_lower));
            }
            check_counts.insert("MetamodelConformance".to_string(), serde_json::json!(findings.len()));
            all_findings.extend(findings);
        }

        let show_detail = req.detail.unwrap_or(false);
        if show_detail {
            serde_json::json!({
                "finding_count": all_findings.len(),
                "counts": check_counts,
                "findings": all_findings.iter().map(|f| serde_json::json!({
                    "check_type": format!("{:?}", f.check_type),
                    "element": f.element,
                    "message": f.message,
                    "file_path": f.file_path,
                    "span": { "start_line": f.span.start_line, "end_line": f.span.end_line },
                })).collect::<Vec<_>>(),
            })
            .to_string()
        } else {
            serde_json::json!({
                "finding_count": all_findings.len(),
                "counts": check_counts,
            })
            .to_string()
        }
    }

    #[tool(description = "Query relationships by predicate: filter by source/target kind, name, and relationship type")]
    async fn sysml_query(&self, #[tool(aggr)] req: QueryRequest) -> String {
        if let Err(e) = self.ensure_graph().await {
            return Self::err_json(&e);
        }

        let graph_guard = self.graph.lock().await;
        let graph = graph_guard.as_ref().unwrap();

        let predicate = Predicate {
            source_kind: req.source_kind,
            source_name: req.source_name,
            relationship_kind: req.rel,
            target_kind: req.target_kind,
            target_name: req.target_name,
            exclude_relationship_kind: req.exclude_rel,
        };

        let mut results = graph.query(predicate);

        let source_layer_filter = req.source_layer.as_deref().and_then(|s| s.parse::<RflpLayer>().ok());
        let target_layer_filter = req.target_layer.as_deref().and_then(|s| s.parse::<RflpLayer>().ok());
        if source_layer_filter.is_some() || target_layer_filter.is_some() {
            let elements = graph.elements();
            results.retain(|t| {
                if let Some(ref sl) = source_layer_filter {
                    let elem = elements.iter().find(|e| e.qualified_name == t.source);
                    if elem.and_then(|e| e.layer.as_ref()) != Some(sl) {
                        return false;
                    }
                }
                if let Some(ref tl) = target_layer_filter {
                    let elem = elements.iter().find(|e| e.qualified_name == t.target);
                    if elem.and_then(|e| e.layer.as_ref()) != Some(tl) {
                        return false;
                    }
                }
                true
            });
        }

        let total = results.len();
        let limit = req.limit.unwrap_or(50);
        let truncated = total > limit;
        results.truncate(limit);

        serde_json::json!({
            "total": total,
            "truncated": truncated,
            "matches": results.iter().map(|t| serde_json::json!({
                "source": t.source,
                "relationship": t.relationship,
                "target": t.target,
                "file_path": t.file_path,
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }

    #[tool(description = "Render a pre-formatted report from the knowledge graph. Returns markdown/html/csv, not JSON. Use for synthesis tasks.")]
    async fn sysml_render(&self, #[tool(aggr)] req: RenderRequest) -> String {
        if let Err(e) = self.ensure_graph().await {
            return Self::err_json(&e);
        }

        let graph_guard = self.graph.lock().await;
        let graph = graph_guard.as_ref().unwrap();

        let builtin = match render::parse_builtin_template(&req.template) {
            Some(t) => t,
            None => {
                return Self::err_json(&format!(
                    "Unknown template '{}'. Valid: traceability-matrix, requirements-table, completeness-report",
                    req.template
                ));
            }
        };

        let fmt = match render::parse_render_format(req.render_format.as_deref().unwrap_or("markdown")) {
            Some(f) => f,
            None => {
                return Self::err_json(&format!(
                    "Unknown format '{}'. Valid: markdown, html, csv",
                    req.render_format.as_deref().unwrap_or("")
                ));
            }
        };

        match render::render_builtin(graph, builtin, fmt) {
            Ok(output) => output,
            Err(e) => Self::err_json(&format!("Render error: {}", e)),
        }
    }

    #[tool(description = "Read the contents of a file. Use this to examine SysML source files, configuration files, or any text file in the workspace.")]
    async fn read_file(&self, #[tool(aggr)] req: ReadFileRequest) -> String {
        let path = PathBuf::from(&req.path);
        if !path.exists() {
            return Self::err_json(&format!("File not found: {}", req.path));
        }
        if !path.is_file() {
            return Self::err_json(&format!("Not a file: {}", req.path));
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => return Self::err_json(&format!("Error reading {}: {}", req.path, e)),
        };

        let lines: Vec<&str> = contents.lines().collect();
        let total_lines = lines.len();
        let offset = req.offset.unwrap_or(1).saturating_sub(1);
        let selected: Vec<&str> = if let Some(limit) = req.limit {
            lines.into_iter().skip(offset).take(limit).collect()
        } else {
            lines.into_iter().skip(offset).collect()
        };

        serde_json::json!({
            "path": path.to_string_lossy(),
            "total_lines": total_lines,
            "offset": offset + 1,
            "lines_returned": selected.len(),
            "content": selected.join("\n"),
        })
        .to_string()
    }

    #[tool(description = "Validate SysML v2 files for syntax errors. Returns diagnostics for each file.")]
    async fn sysml_validate(&self, #[tool(aggr)] req: ValidateRequest) -> String {
        let paths: Vec<PathBuf> = req.paths.iter().map(PathBuf::from).collect();
        let sysml_files = collect_sysml_files(&paths);

        if sysml_files.is_empty() {
            return Self::err_json("No .sysml files found in provided paths");
        }

        let parser = SysmlParser::new();
        let strict = req.strict.unwrap_or(false);
        let mut file_results = Vec::new();
        let mut all_valid = true;

        for file_path in &sysml_files {
            let source = match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(e) => {
                    file_results.push(serde_json::json!({
                        "file": file_path.to_string_lossy(),
                        "valid": false,
                        "diagnostics": [{"severity": "Error", "message": e.to_string()}],
                    }));
                    all_valid = false;
                    continue;
                }
            };

            let diagnostics = parser.validate(&source);
            let has_errors = diagnostics.iter().any(|d| {
                matches!(d.severity, sysml_core::core_types::Severity::Error)
            });
            let has_warnings = diagnostics.iter().any(|d| {
                matches!(d.severity, sysml_core::core_types::Severity::Warning)
            });
            let valid = !has_errors && (!strict || !has_warnings);
            if !valid {
                all_valid = false;
            }

            file_results.push(serde_json::json!({
                "file": file_path.to_string_lossy(),
                "valid": valid,
                "diagnostics": diagnostics.iter().map(|d| serde_json::json!({
                    "severity": format!("{:?}", d.severity),
                    "message": d.message,
                    "span": {
                        "start_line": d.span.start_line,
                        "start_col": d.span.start_col,
                        "end_line": d.span.end_line,
                        "end_col": d.span.end_col,
                    },
                })).collect::<Vec<_>>(),
            }));
        }

        serde_json::json!({
            "files_checked": sysml_files.len(),
            "all_valid": all_valid,
            "results": file_results,
        })
        .to_string()
    }

    #[tool(description = "Inspect an element by exact name. Returns full metadata: kind, layer, members, and all incoming/outgoing relationships. Use for element detail lookups.")]
    async fn sysml_inspect(&self, #[tool(aggr)] req: InspectRequest) -> String {
        if let Err(e) = self.ensure_graph().await {
            return Self::err_json(&e);
        }

        let graph_guard = self.graph.lock().await;
        let graph = graph_guard.as_ref().unwrap();

        match graph.inspect(&req.element) {
            Some(output) => output.to_string(),
            None => Self::err_json(&format!("Element '{}' not found in index", req.element)),
        }
    }

    #[tool(description = "Show model health dashboard: element/relationship counts, type breakdown, completeness score")]
    async fn sysml_stat(&self, #[tool(aggr)] _req: StatRequest) -> String {
        if let Err(e) = self.ensure_graph().await {
            return Self::err_json(&e);
        }

        let graph_guard = self.graph.lock().await;
        let graph = graph_guard.as_ref().unwrap();

        let elements = graph.elements();
        let mut type_breakdown: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        let mut layer_breakdown: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for elem in elements {
            *type_breakdown.entry(&elem.kind).or_insert(0) += 1;
            let layer_label = elem
                .layer
                .as_ref()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "none".to_string());
            *layer_breakdown.entry(layer_label).or_insert(0) += 1;
        }

        let all_checks = vec![
            CheckType::OrphanRequirements,
            CheckType::UnverifiedRequirements,
            CheckType::MissingVerification,
            CheckType::UnconnectedPorts,
            CheckType::DanglingReferences,
        ];

        let mut total_findings = 0;
        let mut check_summaries = Vec::new();
        let mut orphan_count = 0;
        let mut unverified_count = 0;

        for ct in &all_checks {
            let findings = graph.check(ct.clone());
            let count = findings.len();
            total_findings += count;
            match ct {
                CheckType::OrphanRequirements => orphan_count = count,
                CheckType::UnverifiedRequirements => unverified_count = count,
                _ => {}
            }
            check_summaries.push(serde_json::json!({
                "check": format!("{:?}", ct),
                "findings": count,
            }));
        }

        let total_requirements = elements
            .iter()
            .filter(|e| e.kind.to_lowercase().contains("requirement"))
            .count();

        let completeness_score = if total_requirements > 0 {
            let gap = (orphan_count + unverified_count).min(total_requirements);
            1.0 - (gap as f64 / total_requirements as f64)
        } else {
            1.0
        };

        let mut sorted_breakdown: Vec<_> = type_breakdown.into_iter().collect();
        sorted_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

        let mut sorted_layers: Vec<_> = layer_breakdown.into_iter().collect();
        sorted_layers.sort_by(|a, b| b.1.cmp(&a.1));

        let mut rel_breakdown: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for rel in graph.relationships() {
            *rel_breakdown.entry(&rel.kind).or_insert(0) += 1;
        }
        let mut sorted_rels: Vec<_> = rel_breakdown.into_iter().collect();
        sorted_rels.sort_by(|a, b| b.1.cmp(&a.1));

        serde_json::json!({
            "files": graph.file_count(),
            "elements": graph.element_count(),
            "relationships": graph.relationship_count(),
            "completeness_score": (completeness_score * 1000.0).round() / 1000.0,
            "type_breakdown": sorted_breakdown.iter().map(|(k, v)| serde_json::json!({
                "kind": k, "count": v,
            })).collect::<Vec<_>>(),
            "layer_breakdown": sorted_layers.iter().map(|(l, c)| serde_json::json!({
                "layer": l, "count": c,
            })).collect::<Vec<_>>(),
            "relationship_breakdown": sorted_rels.iter().map(|(k, v)| serde_json::json!({
                "kind": k, "count": v,
            })).collect::<Vec<_>>(),
            "checks": check_summaries,
            "total_findings": total_findings,
        })
        .to_string()
    }
}

#[tool(tool_box)]
impl ServerHandler for NomographServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "nomograph-sysml".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "SysML v2 knowledge graph toolkit. Index, search, trace, check, query, and render SysML v2 models. \
                 Start with sysml_index to build the knowledge graph, then use other tools to explore it."
                    .to_string(),
            ),
        }
    }
}

pub async fn run() -> anyhow::Result<()> {
    tracing::info!(
        "Starting nomograph-sysml MCP server v{}",
        env!("CARGO_PKG_VERSION")
    );

    let server = NomographServer::new();
    let transport = rmcp::transport::stdio();
    let server_handle = server.serve(transport).await?;
    server_handle.waiting().await?;

    Ok(())
}
