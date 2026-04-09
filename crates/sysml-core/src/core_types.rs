use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DetailLevel {
    L0,
    L1,
    L2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult<E, R> {
    pub elements: Vec<E>,
    pub relationships: Vec<R>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub qualified_name: String,
    pub kind: String,
    pub file_path: PathBuf,
    pub span: Span,
    pub score: f64,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceOptions {
    pub direction: Direction,
    pub max_hops: u32,
    pub relationship_types: Option<Vec<String>>,
    pub format: TraceFormat,
    #[serde(default)]
    pub include_structural: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TraceFormat {
    Chain,
    Tree,
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResult {
    pub root: String,
    pub hops: Vec<TraceHop>,
    pub format: TraceFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceHop {
    pub depth: u32,
    pub source: String,
    pub relationship: String,
    pub target: String,
    pub file_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_layer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckType {
    OrphanRequirements,
    UnverifiedRequirements,
    MissingVerification,
    UnconnectedPorts,
    DanglingReferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub check_type: CheckType,
    pub element: String,
    pub message: String,
    pub file_path: PathBuf,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predicate {
    pub source_kind: Option<String>,
    pub source_name: Option<String>,
    pub relationship_kind: Option<String>,
    pub target_kind: Option<String>,
    pub target_name: Option<String>,
    #[serde(default)]
    pub exclude_relationship_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Triple {
    pub source: String,
    pub relationship: String,
    pub target: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringCandidate {
    pub qualified_name: String,
    pub kind: String,
    pub file_path: PathBuf,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredResult {
    pub candidate: ScoringCandidate,
    pub score: f64,
    pub signals: Vec<(String, f64)>,
}
