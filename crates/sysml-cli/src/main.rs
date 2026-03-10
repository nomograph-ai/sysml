use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use nomograph_core::traits::{KnowledgeGraph, Parser as NomographParser};
use nomograph_core::types::{
    CheckType, DetailLevel, Direction, Predicate, TraceFormat, TraceOptions,
};
use sysml_core::element::RflpLayer;
use sysml_core::graph::find_index;
use sysml_core::metamodel;
use sysml_core::render;
use sysml_core::{SysmlGraph, SysmlParser};

#[cfg(feature = "mcp")]
mod mcp;

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Pretty,
    Compact,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum CliDetailLevel {
    L0,
    L1,
    L2,
}

impl From<CliDetailLevel> for DetailLevel {
    fn from(v: CliDetailLevel) -> Self {
        match v {
            CliDetailLevel::L0 => DetailLevel::L0,
            CliDetailLevel::L1 => DetailLevel::L1,
            CliDetailLevel::L2 => DetailLevel::L2,
        }
    }
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum CliDirection {
    Forward,
    Backward,
    Both,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum CliTraceFormat {
    Chain,
    Tree,
    Flat,
}

#[derive(Parser)]
#[command(
    name = "nomograph-sysml",
    version,
    about = "CLI-native knowledge graph for SysML v2"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, default_value = "json", global = true, help = "Output format")]
    format: OutputFormat,

    #[arg(long, global = true, help = "Suppress informational output")]
    quiet: bool,

    #[arg(long, global = true, help = "Enable debug-level logging")]
    verbose: bool,

    #[cfg(feature = "mcp")]
    #[arg(long, help = "Start MCP server mode (JSON-RPC over stdio)")]
    mcp: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Parse SysML files and output AST")]
    Parse(ParseArgs),
    #[command(about = "Validate SysML files for syntax errors")]
    Validate(ValidateArgs),
    #[command(about = "Build a knowledge graph index from SysML files")]
    Index(IndexArgs),
    #[command(about = "Search the knowledge graph by name, kind, or text")]
    Search(SearchArgs),
    #[command(about = "Trace relationship chains from an element")]
    Trace(TraceArgs),
    #[command(about = "Run structural completeness checks")]
    Check(CheckArgs),
    #[command(about = "Query relationships by predicate")]
    Query(QueryArgs),
    #[command(about = "Render model reports from templates")]
    Render(RenderArgs),
    #[command(about = "Show model health dashboard")]
    Stat(StatArgs),
    #[command(about = "Inspect an element by exact name with full relationship detail")]
    Inspect(InspectArgs),
    #[command(about = "Decompose a question into executable CLI commands")]
    Plan(PlanArgs),
    #[command(about = "Compare two knowledge graph indexes")]
    Diff(DiffArgs),
    #[command(about = "Generate SysML v2 scaffold text")]
    Scaffold(ScaffoldArgs),
    #[command(about = "Output skill file or generate harness scaffold")]
    Skill(SkillArgs),
}

#[derive(clap::Args)]
struct ParseArgs {
    #[arg(help = "SysML files to parse")]
    files: Vec<PathBuf>,
    #[arg(long, default_value = "l1", help = "Detail level: l0, l1, l2")]
    level: CliDetailLevel,
}

#[derive(clap::Args)]
struct ValidateArgs {
    #[arg(help = "SysML files to validate")]
    files: Vec<PathBuf>,
    #[arg(long, help = "Treat warnings as errors")]
    strict: bool,
}

#[derive(clap::Args)]
struct IndexArgs {
    #[arg(help = "Directories or files to index")]
    paths: Vec<PathBuf>,
    #[arg(long, help = "Output path for the index file")]
    output: Option<PathBuf>,
    #[arg(long, help = "Include vector embeddings (not yet implemented)")]
    vectors: bool,
}

#[derive(clap::Args)]
struct SearchArgs {
    #[arg(help = "Search query (name, kind, or natural language)")]
    query: String,
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
    #[arg(long, default_value = "l1", help = "Detail level: l0, l1, l2")]
    level: CliDetailLevel,
    #[arg(long, default_value = "10", help = "Maximum number of results")]
    limit: usize,
    #[arg(long, help = "Filter results by element kind")]
    kind: Option<String>,
    #[arg(long, help = "Filter by RFLP layer: R, F, L, P")]
    layer: Option<String>,
    #[arg(long, help = "Enable semantic vector search (requires --features vectors)")]
    vectors: bool,
}

#[derive(clap::Args)]
struct TraceArgs {
    #[arg(help = "Starting element name or qualified name")]
    element: String,
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
    #[arg(long, default_value = "3", help = "Maximum traversal depth")]
    hops: u32,
    #[arg(
        long,
        default_value = "both",
        help = "Traversal direction: forward, backward, both"
    )]
    direction: CliDirection,
    #[arg(long, num_args = 0.., help = "Filter by relationship types")]
    types: Vec<String>,
    #[arg(
        long,
        default_value = "chain",
        help = "Output format: chain, tree, flat"
    )]
    trace_format: CliTraceFormat,
    #[arg(long, help = "Maximum number of hops to include in output")]
    max_results: Option<usize>,
    #[arg(long, help = "Include Member and Import edges (structural containment)")]
    include_structural: bool,
}

#[derive(clap::Args)]
struct CheckArgs {
    #[arg(
        help = "Check types to run (or 'all'): orphan-requirements, unverified-requirements, missing-verification, unconnected-ports, dangling-references, metamodel-conformance"
    )]
    check_types: Vec<String>,
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
    #[arg(long, help = "Filter findings by qualified name prefix")]
    scope: Option<String>,
    #[arg(long, help = "Exit with code 1 if any findings are reported")]
    fail_on_findings: bool,
    #[arg(long, help = "Show full findings instead of summary counts")]
    detail: bool,
}

#[derive(clap::Args)]
struct QueryArgs {
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
    #[arg(long, help = "Filter by source element kind")]
    source_kind: Option<String>,
    #[arg(long, help = "Filter by source element name")]
    source_name: Option<String>,
    #[arg(long, help = "Filter by relationship kind")]
    rel: Option<String>,
    #[arg(long, help = "Filter by target element kind")]
    target_kind: Option<String>,
    #[arg(long, help = "Filter by target element name")]
    target_name: Option<String>,
    #[arg(long, default_value = "50", help = "Maximum number of results")]
    limit: usize,
    #[arg(long, help = "One-line compact output: source -> rel -> target")]
    compact: bool,
    #[arg(long, help = "Exclude relationship kinds (comma-separated, e.g., 'member,import')")]
    exclude_rel: Option<String>,
    #[arg(long, help = "Filter by source RFLP layer: R, F, L, P")]
    source_layer: Option<String>,
    #[arg(long, help = "Filter by target RFLP layer: R, F, L, P")]
    target_layer: Option<String>,
}

#[derive(clap::Args)]
struct InspectArgs {
    #[arg(help = "Element name or qualified name to inspect")]
    element: String,
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
}

#[derive(clap::Args)]
struct RenderArgs {
    #[arg(
        long,
        help = "Built-in template: traceability-matrix, requirements-table, completeness-report"
    )]
    template: String,
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
    #[arg(
        long,
        default_value = "markdown",
        help = "Output format: markdown, html, csv"
    )]
    render_format: String,
    #[arg(long, help = "Path to custom Handlebars template file")]
    custom: Option<PathBuf>,
}

#[derive(clap::Args)]
struct StatArgs {
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
    #[arg(long, help = "Output SVG health badge instead of JSON")]
    badge: bool,
}

#[derive(clap::Args)]
struct PlanArgs {
    #[arg(help = "Natural language question about the model")]
    question: String,
    #[arg(
        long,
        default_value = ".nomograph/index.json",
        help = "Path to index file"
    )]
    index: Option<PathBuf>,
    #[arg(long, help = "Execute the plan and return aggregated results")]
    execute: bool,
}

#[derive(clap::Args)]
struct DiffArgs {
    #[arg(help = "Base index file (before changes)")]
    base: PathBuf,
    #[arg(help = "Head index file (after changes)")]
    head: PathBuf,
    #[arg(long, help = "One-line compact output per change")]
    compact: bool,
}

#[derive(clap::Args)]
struct ScaffoldArgs {
    #[arg(help = "Scaffold kind: requirement, verification, part, package, use-case, action, state, interface")]
    kind: String,
    #[arg(help = "Name for the generated element")]
    name: String,
    #[arg(long, help = "Output raw SysML text instead of JSON")]
    raw: bool,
}

#[derive(clap::Args)]
struct SkillArgs {
    #[arg(long, help = "Scan PATH for other nomograph binaries")]
    scan: bool,
    #[arg(long, help = "Generate a full .nomograph/ scaffold")]
    harness: bool,
    #[arg(long, help = "Output directory for scaffold")]
    output: Option<PathBuf>,
}

fn print_json(value: &serde_json::Value, format: &OutputFormat) {
    match format {
        OutputFormat::Pretty => println!("{}", serde_json::to_string_pretty(value).unwrap()),
        OutputFormat::Json | OutputFormat::Compact => {
            println!("{}", serde_json::to_string(value).unwrap())
        }
    }
}

fn element_to_json_l0(elem: &sysml_core::SysmlElement) -> serde_json::Value {
    serde_json::json!({
        "qualified_name": elem.qualified_name,
        "kind": elem.kind,
    })
}

fn element_to_json_l1(elem: &sysml_core::SysmlElement) -> serde_json::Value {
    serde_json::json!({
        "qualified_name": elem.qualified_name,
        "kind": elem.kind,
        "start_line": elem.span.start_line,
        "end_line": elem.span.end_line,
    })
}

fn element_to_json_l2(elem: &sysml_core::SysmlElement) -> serde_json::Value {
    serde_json::json!({
        "qualified_name": elem.qualified_name,
        "kind": elem.kind,
        "file_path": elem.file_path,
        "span": {
            "start_line": elem.span.start_line,
            "start_col": elem.span.start_col,
            "end_line": elem.span.end_line,
            "end_col": elem.span.end_col,
        },
        "doc": elem.doc,
        "attributes": elem.attributes,
    })
}

fn relationship_to_json(
    rel: &sysml_core::SysmlRelationship,
    level: &DetailLevel,
) -> serde_json::Value {
    match level {
        DetailLevel::L0 => serde_json::json!({
            "source": rel.source,
            "kind": rel.kind,
            "target": rel.target,
        }),
        DetailLevel::L1 => serde_json::json!({
            "source": rel.source,
            "kind": rel.kind,
            "target": rel.target,
            "start_line": rel.span.start_line,
        }),
        DetailLevel::L2 => serde_json::json!({
            "source": rel.source,
            "kind": rel.kind,
            "target": rel.target,
            "file_path": rel.file_path,
            "span": {
                "start_line": rel.span.start_line,
                "start_col": rel.span.start_col,
                "end_line": rel.span.end_line,
                "end_col": rel.span.end_col,
            },
        }),
    }
}

fn run_parse(args: ParseArgs, format: &OutputFormat) -> i32 {
    let parser = SysmlParser::new();
    let level: DetailLevel = args.level.into();
    let mut exit_code = 0;

    for file_path in &args.files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {}: {}", file_path.display(), e);
                return 2;
            }
        };

        let result = match parser.parse(&source, file_path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: {}: {}", file_path.display(), e);
                return 2;
            }
        };

        if result
            .diagnostics
            .iter()
            .any(|d| d.severity == nomograph_core::types::Severity::Error)
        {
            exit_code = 1;
        }

        let elements: Vec<serde_json::Value> = result
            .elements
            .iter()
            .map(|e| match level {
                DetailLevel::L0 => element_to_json_l0(e),
                DetailLevel::L1 => element_to_json_l1(e),
                DetailLevel::L2 => element_to_json_l2(e),
            })
            .collect();

        let relationships: Vec<serde_json::Value> = result
            .relationships
            .iter()
            .map(|r| relationship_to_json(r, &level))
            .collect();

        let diagnostics: Vec<serde_json::Value> = result
            .diagnostics
            .iter()
            .map(|d| {
                serde_json::json!({
                    "severity": format!("{:?}", d.severity),
                    "message": d.message,
                    "span": {
                        "start_line": d.span.start_line,
                        "start_col": d.span.start_col,
                        "end_line": d.span.end_line,
                        "end_col": d.span.end_col,
                    },
                })
            })
            .collect();

        let output = serde_json::json!({
            "file": file_path.to_string_lossy(),
            "elements": elements,
            "relationships": relationships,
            "diagnostics": diagnostics,
        });

        print_json(&output, format);
    }

    exit_code
}

fn run_validate(args: ValidateArgs, format: &OutputFormat) -> i32 {
    let parser = SysmlParser::new();
    let mut exit_code = 0;

    for file_path in &args.files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {}: {}", file_path.display(), e);
                return 2;
            }
        };

        let diagnostics = parser.validate(&source);

        let has_errors = diagnostics.iter().any(|d| {
            d.severity == nomograph_core::types::Severity::Error
                || (args.strict && d.severity == nomograph_core::types::Severity::Warning)
        });

        if has_errors {
            exit_code = 1;
        }

        let diag_json: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(|d| {
                serde_json::json!({
                    "severity": format!("{:?}", d.severity),
                    "message": d.message,
                    "span": {
                        "start_line": d.span.start_line,
                        "start_col": d.span.start_col,
                        "end_line": d.span.end_line,
                        "end_col": d.span.end_col,
                    },
                })
            })
            .collect();

        let output = serde_json::json!({
            "file": file_path.to_string_lossy(),
            "valid": !has_errors,
            "diagnostics": diag_json,
        });

        print_json(&output, format);
    }

    exit_code
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

fn run_index(args: IndexArgs, format: &OutputFormat) -> i32 {
    let parser = SysmlParser::new();
    let sysml_files = collect_sysml_files(&args.paths);

    if sysml_files.is_empty() {
        eprintln!("error: no .sysml files found in the provided paths");
        return 2;
    }

    let mut results = Vec::new();
    for file_path in &sysml_files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {}: {}", file_path.display(), e);
                return 2;
            }
        };
        match parser.parse(&source, file_path) {
            Ok(r) => results.push(r),
            Err(e) => {
                eprintln!("error: {}: {}", file_path.display(), e);
                return 2;
            }
        }
    }

    let mut graph = SysmlGraph::new();
    if let Err(e) = graph.index(results) {
        eprintln!("error: indexing failed: {}", e);
        return 1;
    }

    #[cfg(feature = "vectors")]
    let vectors_built = if args.vectors {
        eprintln!("Building vector embeddings...");
        match graph.build_vectors() {
            Ok(()) => true,
            Err(e) => {
                eprintln!("warning: vector index failed: {e}");
                false
            }
        }
    } else {
        false
    };
    #[cfg(not(feature = "vectors"))]
    let vectors_built = false;
    if args.vectors && !vectors_built {
        #[cfg(not(feature = "vectors"))]
        eprintln!("warning: --vectors requires --features vectors at build time");
    }

    let index_path = match &args.output {
        Some(p) => p.clone(),
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            cwd.join(".nomograph").join("index.json")
        }
    };

    if let Err(e) = graph.save(&index_path) {
        eprintln!("error: saving index: {}", e);
        return 1;
    }

    let output = serde_json::json!({
        "files_indexed": graph.file_count(),
        "elements": graph.element_count(),
        "relationships": graph.relationship_count(),
        "vectors": vectors_built,
        "index_path": index_path.to_string_lossy(),
    });

    print_json(&output, format);
    0
}

fn resolve_index_path(specified: &Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = specified {
        if p.to_string_lossy() != ".nomograph/index.json" || p.exists() {
            return Some(p.clone());
        }
    }
    let cwd = std::env::current_dir().ok()?;
    find_index(&cwd)
}

fn run_search(args: SearchArgs, format: &OutputFormat) -> i32 {
    let index_path = match resolve_index_path(&args.index) {
        Some(p) => p,
        None => {
            eprintln!("error: index not found. Run `nomograph-sysml index` first.");
            return 3;
        }
    };

    #[allow(unused_mut)]
    let mut graph = match SysmlGraph::load(&index_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: loading index: {}", e);
            return 3;
        }
    };

    #[cfg(feature = "vectors")]
    if args.vectors {
        eprintln!("Building vector embeddings for search...");
        if let Err(e) = graph.build_vectors() {
            eprintln!("warning: vector index failed: {e}");
        }
    }
    #[cfg(not(feature = "vectors"))]
    if args.vectors {
        eprintln!("warning: --vectors requires --features vectors at build time");
    }

    let level: DetailLevel = args.level.into();
    let total_candidates = graph.element_count();
    let mut results = graph.search(&args.query, level, args.limit);

    if let Some(kind_filter) = &args.kind {
        results.retain(|r| &r.kind == kind_filter);
    }

    if let Some(ref layer_str) = args.layer {
        match layer_str.parse::<RflpLayer>() {
            Ok(target_layer) => {
                let elements = graph.elements();
                results.retain(|r| {
                    elements
                        .iter()
                        .find(|e| e.qualified_name == r.qualified_name)
                        .and_then(|e| e.layer.as_ref())
                        .is_some_and(|l| *l == target_layer)
                });
            }
            Err(e) => eprintln!("warning: {}", e),
        }
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

    let output = serde_json::json!({
        "total_candidates": total_candidates,
        "results_returned": items.len(),
        "results": items,
    });

    print_json(&output, format);
    0
}

fn run_trace(args: TraceArgs, format: &OutputFormat) -> i32 {
    let index_path = match resolve_index_path(&args.index) {
        Some(p) => p,
        None => {
            eprintln!("error: index not found. Run `nomograph-sysml index` first.");
            return 3;
        }
    };

    let graph = match SysmlGraph::load(&index_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: loading index: {}", e);
            return 3;
        }
    };

    let direction = match args.direction {
        CliDirection::Forward => Direction::Forward,
        CliDirection::Backward => Direction::Backward,
        CliDirection::Both => Direction::Both,
    };

    let trace_format = match args.trace_format {
        CliTraceFormat::Chain => TraceFormat::Chain,
        CliTraceFormat::Tree => TraceFormat::Tree,
        CliTraceFormat::Flat => TraceFormat::Flat,
    };

    let relationship_types = if args.types.is_empty() {
        None
    } else {
        Some(args.types)
    };

    let include_structural =
        args.include_structural || relationship_types.as_ref().is_some_and(|types| {
            types.iter().any(|t| {
                let tl = t.to_lowercase();
                tl == "member" || tl == "import"
            })
        });

    let opts = TraceOptions {
        direction,
        max_hops: args.hops,
        relationship_types,
        format: trace_format,
        include_structural,
    };

    let result = graph.trace(&args.element, opts);

    let total_hops = result.hops.len();
    let truncated = args.max_results.is_some_and(|max| total_hops > max);
    let hops: Vec<_> = if let Some(max) = args.max_results {
        result.hops.iter().take(max).collect()
    } else {
        result.hops.iter().collect()
    };

    let output = serde_json::json!({
        "root": result.root,
        "format": format!("{:?}", result.format),
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
    });

    print_json(&output, format);
    0
}

fn parse_check_type(s: &str) -> Option<CheckTypeOrMetamodel> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "orphan_requirements" => Some(CheckTypeOrMetamodel::Structural(
            CheckType::OrphanRequirements,
        )),
        "unverified_requirements" => Some(CheckTypeOrMetamodel::Structural(
            CheckType::UnverifiedRequirements,
        )),
        "missing_verification" => Some(CheckTypeOrMetamodel::Structural(
            CheckType::MissingVerification,
        )),
        "unconnected_ports" => Some(CheckTypeOrMetamodel::Structural(
            CheckType::UnconnectedPorts,
        )),
        "dangling_references" => Some(CheckTypeOrMetamodel::Structural(
            CheckType::DanglingReferences,
        )),
        "metamodel_conformance" | "metamodel" => Some(CheckTypeOrMetamodel::Metamodel),
        _ => None,
    }
}

enum CheckTypeOrMetamodel {
    Structural(CheckType),
    Metamodel,
}

fn run_check(args: CheckArgs, format: &OutputFormat) -> i32 {
    let index_path = match resolve_index_path(&args.index) {
        Some(p) => p,
        None => {
            eprintln!("error: index not found. Run `nomograph-sysml index` first.");
            return 3;
        }
    };

    let graph = match SysmlGraph::load(&index_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: loading index: {}", e);
            return 3;
        }
    };

    let mut include_metamodel = false;
    let check_types: Vec<CheckType> = if args.check_types.is_empty()
        || args.check_types.iter().any(|c| c == "all")
    {
        include_metamodel = args.check_types.iter().any(|c| c == "all");
        vec![
            CheckType::OrphanRequirements,
            CheckType::UnverifiedRequirements,
            CheckType::MissingVerification,
            CheckType::UnconnectedPorts,
            CheckType::DanglingReferences,
        ]
    } else {
        let mut types = Vec::new();
        for ct in &args.check_types {
            match parse_check_type(ct) {
                Some(CheckTypeOrMetamodel::Structural(t)) => types.push(t),
                Some(CheckTypeOrMetamodel::Metamodel) => include_metamodel = true,
                None => {
                    eprintln!("error: unknown check type '{}'. Valid: orphan-requirements, unverified-requirements, missing-verification, unconnected-ports, dangling-references, metamodel-conformance, all", ct);
                    return 1;
                }
            }
        }
        types
    };

    let mut all_findings = Vec::new();
    let mut check_counts: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

    for ct in &check_types {
        let label = format!("{:?}", ct);
        let mut findings = graph.check(ct.clone());
        if let Some(ref scope) = args.scope {
            let scope_lower = scope.to_lowercase();
            findings.retain(|f| f.element.to_lowercase().starts_with(&scope_lower));
        }
        check_counts.insert(label, serde_json::json!(findings.len()));
        all_findings.extend(findings);
    }

    if include_metamodel {
        let mut findings = metamodel::run_metamodel_checks(&graph);
        if let Some(ref scope) = args.scope {
            let scope_lower = scope.to_lowercase();
            findings.retain(|f| f.element.to_lowercase().starts_with(&scope_lower));
        }
        check_counts.insert(
            "MetamodelConformance".to_string(),
            serde_json::json!(findings.len()),
        );
        all_findings.extend(findings);
    }

    let output = if args.detail {
        serde_json::json!({
            "finding_count": all_findings.len(),
            "counts": check_counts,
            "findings": all_findings.iter().map(|f| serde_json::json!({
                "check_type": format!("{:?}", f.check_type),
                "element": f.element,
                "message": f.message,
                "file_path": f.file_path,
                "span": {
                    "start_line": f.span.start_line,
                    "end_line": f.span.end_line,
                },
            })).collect::<Vec<_>>(),
        })
    } else {
        serde_json::json!({
            "finding_count": all_findings.len(),
            "counts": check_counts,
        })
    };

    print_json(&output, format);

    if args.fail_on_findings && !all_findings.is_empty() {
        return 1;
    }
    0
}

fn run_query(args: QueryArgs, format: &OutputFormat) -> i32 {
    let index_path = match resolve_index_path(&args.index) {
        Some(p) => p,
        None => {
            eprintln!("error: index not found. Run `nomograph-sysml index` first.");
            return 3;
        }
    };

    let graph = match SysmlGraph::load(&index_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: loading index: {}", e);
            return 3;
        }
    };

    let predicate = Predicate {
        source_kind: args.source_kind,
        source_name: args.source_name,
        relationship_kind: args.rel,
        target_kind: args.target_kind,
        target_name: args.target_name,
        exclude_relationship_kind: args.exclude_rel,
    };

    let mut results = graph.query(predicate);

    let source_layer_filter = args.source_layer.as_deref().and_then(|s| s.parse::<RflpLayer>().ok());
    let target_layer_filter = args.target_layer.as_deref().and_then(|s| s.parse::<RflpLayer>().ok());
    if source_layer_filter.is_some() || target_layer_filter.is_some() {
        let elements = graph.elements();
        results.retain(|t| {
            if let Some(ref sl) = source_layer_filter {
                let ok = elements
                    .iter()
                    .find(|e| e.qualified_name == t.source)
                    .and_then(|e| e.layer.as_ref())
                    .is_some_and(|l| l == sl);
                if !ok {
                    return false;
                }
            }
            if let Some(ref tl) = target_layer_filter {
                let ok = elements
                    .iter()
                    .find(|e| e.qualified_name == t.target)
                    .and_then(|e| e.layer.as_ref())
                    .is_some_and(|l| l == tl);
                if !ok {
                    return false;
                }
            }
            true
        });
    }

    let total = results.len();
    let truncated = total > args.limit;
    results.truncate(args.limit);

    if args.compact {
        let output = serde_json::json!({
            "total": total,
            "truncated": truncated,
            "matches": results.iter().map(|t| {
                format!("{} -> {} -> {} ({})", t.source, t.relationship, t.target, t.file_path.display())
            }).collect::<Vec<_>>(),
        });
        print_json(&output, format);
    } else {
        let output = serde_json::json!({
            "total": total,
            "truncated": truncated,
            "matches": results.iter().map(|t| serde_json::json!({
                "source": t.source,
                "relationship": t.relationship,
                "target": t.target,
                "file_path": t.file_path,
            })).collect::<Vec<_>>(),
        });
        print_json(&output, format);
    }
    0
}

const SKILL_MD: &str = r#"# nomograph-sysml (v0.1.0)

SysML v2 knowledge graph toolkit. Parse, index, search, analyze, and render SysML v2 models.
Also available as MCP server via `nomograph-sysml --mcp`.

## Available Commands

| Command | Purpose | Example |
|---------|---------|---------|
| parse | Parse SysML files to AST | `nomograph-sysml parse *.sysml --level l1` |
| validate | Check files for errors | `nomograph-sysml validate model.sysml` |
| index | Build knowledge graph | `nomograph-sysml index ./model/` |
| search | Search by name/kind/text | `nomograph-sysml search "requirement" --kind requirement_definition` |
| trace | Follow relationships | `nomograph-sysml trace ShieldModule --hops 3 --direction both` |
| check | Structural + metamodel checks | `nomograph-sysml check all` or `check metamodel-conformance` |
| query | Predicate relationship search | `nomograph-sysml query --rel satisfy --source-name "shield"` |
| render | Template-based reports | `nomograph-sysml render --template traceability-matrix` |
| stat | Model health dashboard | `nomograph-sysml stat` |
| plan | Decompose question into commands | `nomograph-sysml plan "Does X satisfy Y?"` |
| skill | Agent skill file | `nomograph-sysml skill` |

## Typical Workflow

1. `nomograph-sysml index ./model/` — build knowledge graph
2. `nomograph-sysml search "<what you need>"` — find relevant elements
3. `nomograph-sysml trace <element> --hops 3` — follow impact chains
4. `nomograph-sysml check all` — find structural + metamodel gaps
5. `nomograph-sysml render --template completeness-report` — generate report

## Key Options

- `check --detail` — full findings instead of summary counts
- `trace --max-results N` — limit output for token efficiency
- `query --compact` — one-line per relationship
- `render --render-format html|csv` — alternate output formats
- `render --custom path.hbs` — custom Handlebars template
- `search --layer R|F|L|P` — filter by RFLP architecture layer
- `query --source-layer R --target-layer P` — cross-layer relationship queries
- `plan --execute` — run decomposed plan and aggregate results

## Output

All commands output JSON by default. `render` outputs markdown/html/csv.
Pipe between commands: `nomograph-sysml search "port" | jq '.[].qualified_name'`

## Help

Run `nomograph-sysml <command> --help` for detailed options.
"#;

const CONFIG_TOML: &str = r#"# nomograph-sysml configuration

[index]
path = ".nomograph/index.json"
auto_index = true

[output]
format = "json"
detail_level = "l1"

[search]
default_limit = 10

[trace]
max_hops = 3
direction = "both"
format = "chain"

[check]
default_checks = ["all"]
"#;

const SCRIPT_ANALYZE: &str = r#"#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="${1:-.}"
INDEX_PATH="${2:-.nomograph/index.json}"

echo "=== Indexing $MODEL_DIR ===" >&2
nomograph-sysml index "$MODEL_DIR" --output "$INDEX_PATH"

echo "=== Running all checks ===" >&2
nomograph-sysml check all --index "$INDEX_PATH"
"#;

const SCRIPT_IMPACT: &str = r#"#!/usr/bin/env bash
set -euo pipefail

ELEMENT="${1:?Usage: impact.sh <element> [hops] [index-path]}"
HOPS="${2:-3}"
INDEX_PATH="${3:-.nomograph/index.json}"

nomograph-sysml trace "$ELEMENT" --index "$INDEX_PATH" --hops "$HOPS" --direction both --trace-format flat
"#;

const SCRIPT_VALIDATE_MODEL: &str = r#"#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="${1:-.}"
ERRORS=0

for f in $(find "$MODEL_DIR" -name '*.sysml' -type f); do
    RESULT=$(nomograph-sysml validate "$f" 2>/dev/null)
    VALID=$(echo "$RESULT" | jq -r '.valid')
    if [ "$VALID" != "true" ]; then
        echo "INVALID: $f" >&2
        echo "$RESULT" | jq '.diagnostics[]' >&2
        ERRORS=$((ERRORS + 1))
    fi
done

if [ "$ERRORS" -eq 0 ]; then
    echo '{"valid":true,"files_checked":"all"}' 
else
    echo "{\"valid\":false,\"errors\":$ERRORS}" 
    exit 1
fi
"#;

fn scan_nomograph_binaries() -> Vec<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for dir in path_var.split(':') {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("nomograph-") && !seen.contains(name) {
                        seen.insert(name.to_string());
                        found.push(name.to_string());
                    }
                }
            }
        }
    }

    found.sort();
    found
}

fn run_diff(args: DiffArgs, format: &OutputFormat) -> i32 {
    let base = match SysmlGraph::load(&args.base) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to load base index: {e}");
            return 2;
        }
    };
    let head = match SysmlGraph::load(&args.head) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to load head index: {e}");
            return 2;
        }
    };
    let result = sysml_core::diff::diff_graphs(&base, &head);
    if args.compact {
        let lines = sysml_core::diff::format_compact(&result);
        let output = serde_json::json!({
            "changes": lines,
            "summary": result.summary,
        });
        print_json(&output, format);
    } else {
        let output = serde_json::to_value(&result).unwrap_or_default();
        print_json(&output, format);
    }
    0
}

fn run_scaffold(args: ScaffoldArgs, format: &OutputFormat) -> i32 {
    let kind = match args.kind.parse::<sysml_core::scaffold::ScaffoldKind>() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let result = sysml_core::scaffold::generate(kind, &args.name);
    if args.raw {
        println!("{}", result.sysml);
    } else {
        let output = serde_json::to_value(&result).unwrap_or_default();
        print_json(&output, format);
    }
    0
}

fn run_skill(args: SkillArgs, format: &OutputFormat) -> i32 {
    if args.harness {
        return run_skill_harness(&args, format);
    }

    if args.scan {
        let binaries = scan_nomograph_binaries();
        let output = serde_json::json!({
            "nomograph_binaries": binaries,
        });
        print_json(&output, format);
        return 0;
    }

    print!("{}", SKILL_MD);
    0
}

fn run_skill_harness(args: &SkillArgs, format: &OutputFormat) -> i32 {
    let output_dir = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(".nomograph"));

    let scripts_dir = output_dir.join("scripts");

    if let Err(e) = std::fs::create_dir_all(&scripts_dir) {
        eprintln!("error: creating directories: {}", e);
        return 2;
    }

    let mut created = Vec::new();

    let skill_path = output_dir.join("SKILL.md");
    let mut skill_content = SKILL_MD.to_string();

    if args.scan {
        let binaries = scan_nomograph_binaries();
        if !binaries.is_empty() {
            skill_content.push_str("\n## Other nomograph tools on this host\n\n");
            for bin in &binaries {
                if bin != "nomograph-sysml" {
                    skill_content.push_str(&format!("- `{bin}` — run `{bin} skill` for details\n"));
                }
            }
            skill_content.push('\n');
        }
    }

    let config_path = output_dir.join("config.toml");
    let analyze_path = scripts_dir.join("analyze.sh");
    let impact_path = scripts_dir.join("impact.sh");
    let validate_path = scripts_dir.join("validate-model.sh");

    let files: &[(&std::path::Path, &str, bool)] = &[
        (skill_path.as_ref(), &skill_content, false),
        (config_path.as_ref(), CONFIG_TOML, false),
        (analyze_path.as_ref(), SCRIPT_ANALYZE, true),
        (impact_path.as_ref(), SCRIPT_IMPACT, true),
        (validate_path.as_ref(), SCRIPT_VALIDATE_MODEL, true),
    ];

    for &(path, content, executable) in files {
        if let Err(e) = std::fs::write(path, content) {
            eprintln!("error: writing {}: {}", path.display(), e);
            return 2;
        }
        if executable {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
            }
        }
        created.push(path.to_string_lossy().to_string());
    }

    let output = serde_json::json!({
        "harness_dir": output_dir.to_string_lossy(),
        "files_created": created,
    });

    print_json(&output, format);
    0
}

fn run_render(args: RenderArgs, _format: &OutputFormat) -> i32 {
    let index_path = match resolve_index_path(&args.index) {
        Some(p) => p,
        None => {
            eprintln!("error: index not found. Run `nomograph-sysml index` first.");
            return 3;
        }
    };

    let graph = match SysmlGraph::load(&index_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: loading index: {}", e);
            return 3;
        }
    };

    if let Some(custom_path) = &args.custom {
        match render::render_custom(&graph, custom_path) {
            Ok(output) => {
                print!("{}", output);
                return 0;
            }
            Err(e) => {
                eprintln!("error: rendering custom template: {}", e);
                return 1;
            }
        }
    }

    let builtin = match render::parse_builtin_template(&args.template) {
        Some(t) => t,
        None => {
            eprintln!(
                "error: unknown template '{}'. Valid: traceability-matrix, requirements-table, completeness-report",
                args.template
            );
            return 1;
        }
    };

    let fmt = match render::parse_render_format(&args.render_format) {
        Some(f) => f,
        None => {
            eprintln!(
                "error: unknown render format '{}'. Valid: markdown, html, csv",
                args.render_format
            );
            return 1;
        }
    };

    match render::render_builtin(&graph, builtin, fmt) {
        Ok(output) => {
            print!("{}", output);
            0
        }
        Err(e) => {
            eprintln!("error: rendering template: {}", e);
            1
        }
    }
}

fn run_plan(args: PlanArgs, format: &OutputFormat) -> i32 {
    let index_path = resolve_index_path(&args.index)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".nomograph/index.json".to_string());

    let steps = sysml_core::plan::decompose(&args.question, &index_path);
    let question_type = sysml_core::plan::classify_question(&args.question);

    if !args.execute {
        let output = serde_json::json!({
            "question": args.question,
            "question_type": format!("{:?}", question_type),
            "steps": steps,
        });
        print_json(&output, format);
        return 0;
    }

    let self_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("nomograph-sysml"));

    let mut step_results = Vec::new();
    for step in &steps {
        eprintln!("Step {}: {}", step.step, step.purpose);
        eprintln!("  $ {}", step.command);

        let parts: Vec<&str> = step.command.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let exe = if parts[0] == "nomograph-sysml" {
            self_exe.to_string_lossy().to_string()
        } else {
            parts[0].to_string()
        };

        let output = std::process::Command::new(&exe)
            .args(&parts[1..])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                let parsed: serde_json::Value =
                    serde_json::from_str(&stdout).unwrap_or(serde_json::json!(stdout.trim()));
                step_results.push(serde_json::json!({
                    "step": step.step,
                    "command": step.command,
                    "purpose": step.purpose,
                    "exit_code": result.status.code(),
                    "result": parsed,
                }));
            }
            Err(e) => {
                step_results.push(serde_json::json!({
                    "step": step.step,
                    "command": step.command,
                    "purpose": step.purpose,
                    "error": e.to_string(),
                }));
            }
        }
    }

    let output = serde_json::json!({
        "question": args.question,
        "question_type": format!("{:?}", question_type),
        "steps_executed": step_results.len(),
        "results": step_results,
    });
    print_json(&output, format);
    0
}

fn run_inspect(args: InspectArgs, format: &OutputFormat) -> i32 {
    let index_path = match resolve_index_path(&args.index) {
        Some(p) => p,
        None => {
            eprintln!("error: index not found. Run `nomograph-sysml index` first.");
            return 3;
        }
    };

    let graph = match SysmlGraph::load(&index_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: loading index: {}", e);
            return 3;
        }
    };

    match graph.inspect(&args.element) {
        Some(output) => {
            print_json(&output, format);
            0
        }
        None => {
            let output = serde_json::json!({
                "error": format!("Element '{}' not found in index", args.element),
            });
            print_json(&output, format);
            1
        }
    }
}

fn run_stat(args: StatArgs, format: &OutputFormat) -> i32 {
    let index_path = match resolve_index_path(&args.index) {
        Some(p) => p,
        None => {
            eprintln!("error: index not found. Run `nomograph-sysml index` first.");
            return 3;
        }
    };

    let graph = match SysmlGraph::load(&index_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: loading index: {}", e);
            return 3;
        }
    };

    if args.badge {
        let data = sysml_core::badge::compute_badge_data(&graph);
        print!("{}", sysml_core::badge::render_svg(&data));
        return 0;
    }

    let elements = graph.elements();
    let element_count = elements.len();
    let relationship_count = graph.relationship_count();

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

    let mut rel_breakdown: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for rel in graph.relationships() {
        *rel_breakdown.entry(&rel.kind).or_insert(0) += 1;
    }

    let file_count = graph.file_count();

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

    let mut sorted_rels: Vec<_> = rel_breakdown.into_iter().collect();
    sorted_rels.sort_by(|a, b| b.1.cmp(&a.1));

    let output = serde_json::json!({
        "files": file_count,
        "elements": element_count,
        "relationships": relationship_count,
        "completeness_score": (completeness_score * 1000.0).round() / 1000.0,
        "type_breakdown": sorted_breakdown.iter().map(|(k, v)| serde_json::json!({
            "kind": k,
            "count": v,
        })).collect::<Vec<_>>(),
        "layer_breakdown": sorted_layers.iter().map(|(l, c)| serde_json::json!({
            "layer": l,
            "count": c,
        })).collect::<Vec<_>>(),
        "relationship_breakdown": sorted_rels.iter().map(|(k, v)| serde_json::json!({
            "kind": k,
            "count": v,
        })).collect::<Vec<_>>(),
        "checks": check_summaries,
        "total_findings": total_findings,
    });

    print_json(&output, format);
    0
}

fn main() {
    let cli = Cli::parse();

    #[cfg(feature = "mcp")]
    if cli.mcp {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(std::io::stderr)
            .init();

        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        if let Err(e) = rt.block_on(mcp::run()) {
            eprintln!("error: MCP server failed: {}", e);
            process::exit(1);
        }
        return;
    }

    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else if cli.quiet {
        tracing::Level::ERROR
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .init();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            eprintln!("error: no command provided. Run `nomograph-sysml --help` for usage.");
            process::exit(1);
        }
    };

    let exit_code = match command {
        Commands::Parse(args) => run_parse(args, &cli.format),
        Commands::Validate(args) => run_validate(args, &cli.format),
        Commands::Index(args) => run_index(args, &cli.format),
        Commands::Search(args) => run_search(args, &cli.format),
        Commands::Trace(args) => run_trace(args, &cli.format),
        Commands::Check(args) => run_check(args, &cli.format),
        Commands::Query(args) => run_query(args, &cli.format),
        Commands::Inspect(args) => run_inspect(args, &cli.format),
        Commands::Render(args) => run_render(args, &cli.format),
        Commands::Stat(args) => run_stat(args, &cli.format),
        Commands::Diff(args) => run_diff(args, &cli.format),
        Commands::Scaffold(args) => run_scaffold(args, &cli.format),
        Commands::Plan(args) => run_plan(args, &cli.format),
        Commands::Skill(args) => run_skill(args, &cli.format),
    };

    process::exit(exit_code);
}
