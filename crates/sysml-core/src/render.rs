use std::collections::HashMap;
use std::path::Path;

use handlebars::Handlebars;
use serde::Serialize;

use crate::element::SysmlElement;
use crate::graph::SysmlGraph;
use crate::relationship::SysmlRelationship;
use crate::core_traits::KnowledgeGraph;
use crate::core_types::CheckType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTemplate {
    TraceabilityMatrix,
    RequirementsTable,
    CompletenessReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    Markdown,
    Html,
    Csv,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("template error: {0}")]
    Template(#[from] handlebars::RenderError),
    #[error("template parse error: {0}")]
    TemplateParse(#[from] Box<handlebars::TemplateError>),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Serialize)]
struct TraceabilityRow {
    requirement: String,
    kind: String,
    file: String,
    satisfied_by: Vec<String>,
    verified_by: Vec<String>,
    status: String,
}

#[derive(Serialize)]
struct TraceabilityContext {
    rows: Vec<TraceabilityRow>,
    total_requirements: usize,
    satisfied_count: usize,
    verified_count: usize,
    coverage_pct: String,
}

#[derive(Serialize)]
struct RequirementRow {
    name: String,
    kind: String,
    file: String,
    line: u32,
    doc: String,
    member_count: usize,
}

#[derive(Serialize)]
struct RequirementsContext {
    rows: Vec<RequirementRow>,
    total: usize,
}

#[derive(Serialize)]
struct CheckSummary {
    name: String,
    count: usize,
}

#[derive(Serialize)]
struct CompletenessContext {
    files: usize,
    elements: usize,
    relationships: usize,
    completeness_score: String,
    checks: Vec<CheckSummary>,
    total_findings: usize,
    type_breakdown: Vec<TypeCount>,
}

#[derive(Serialize)]
struct TypeCount {
    kind: String,
    count: usize,
}

const TRACEABILITY_MATRIX_MD: &str = r#"# Traceability Matrix

| Requirement | Kind | Satisfied By | Verified By | Status |
|-------------|------|-------------|-------------|--------|
{{#each rows}}
| {{requirement}} | {{kind}} | {{#each satisfied_by}}{{this}}{{#unless @last}}, {{/unless}}{{/each}} | {{#each verified_by}}{{this}}{{#unless @last}}, {{/unless}}{{/each}} | {{status}} |
{{/each}}

**Summary**: {{total_requirements}} requirements, {{satisfied_count}} satisfied, {{verified_count}} verified ({{coverage_pct}}% coverage)
"#;

const TRACEABILITY_MATRIX_HTML: &str = r#"<html><head><title>Traceability Matrix</title>
<style>table{border-collapse:collapse;width:100%}th,td{border:1px solid #ddd;padding:8px;text-align:left}th{background:#f4f4f4}.gap{background:#fee}.ok{background:#efe}</style>
</head><body>
<h1>Traceability Matrix</h1>
<table><thead><tr><th>Requirement</th><th>Kind</th><th>Satisfied By</th><th>Verified By</th><th>Status</th></tr></thead><tbody>
{{#each rows}}
<tr class="{{#if (eq status "gap")}}gap{{else}}ok{{/if}}"><td>{{requirement}}</td><td>{{kind}}</td><td>{{#each satisfied_by}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}</td><td>{{#each verified_by}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}</td><td>{{status}}</td></tr>
{{/each}}
</tbody></table>
<p><strong>Summary</strong>: {{total_requirements}} requirements, {{satisfied_count}} satisfied, {{verified_count}} verified ({{coverage_pct}}% coverage)</p>
</body></html>"#;

const TRACEABILITY_MATRIX_CSV: &str = r#"Requirement,Kind,Satisfied By,Verified By,Status
{{#each rows}}
{{requirement}},{{kind}},"{{#each satisfied_by}}{{this}}{{#unless @last}}; {{/unless}}{{/each}}","{{#each verified_by}}{{this}}{{#unless @last}}; {{/unless}}{{/each}}",{{status}}
{{/each}}"#;

const REQUIREMENTS_TABLE_MD: &str = r#"# Requirements Table

| # | Requirement | Kind | File | Line | Doc | Members |
|---|-------------|------|------|------|-----|---------|
{{#each rows}}
| {{@index}} | {{name}} | {{kind}} | {{file}} | {{line}} | {{doc}} | {{member_count}} |
{{/each}}

**Total**: {{total}} requirements
"#;

const REQUIREMENTS_TABLE_HTML: &str = r#"<html><head><title>Requirements Table</title>
<style>table{border-collapse:collapse;width:100%}th,td{border:1px solid #ddd;padding:8px;text-align:left}th{background:#f4f4f4}</style>
</head><body>
<h1>Requirements Table</h1>
<table><thead><tr><th>#</th><th>Requirement</th><th>Kind</th><th>File</th><th>Line</th><th>Doc</th><th>Members</th></tr></thead><tbody>
{{#each rows}}
<tr><td>{{@index}}</td><td>{{name}}</td><td>{{kind}}</td><td>{{file}}</td><td>{{line}}</td><td>{{doc}}</td><td>{{member_count}}</td></tr>
{{/each}}
</tbody></table>
<p><strong>Total</strong>: {{total}} requirements</p>
</body></html>"#;

const REQUIREMENTS_TABLE_CSV: &str = r#"#,Requirement,Kind,File,Line,Doc,Members
{{#each rows}}
{{@index}},{{name}},{{kind}},{{file}},{{line}},"{{doc}}",{{member_count}}
{{/each}}"#;

const COMPLETENESS_REPORT_MD: &str = r#"# Model Completeness Report

## Overview

| Metric | Value |
|--------|-------|
| Files | {{files}} |
| Elements | {{elements}} |
| Relationships | {{relationships}} |
| Completeness Score | {{completeness_score}} |
| Total Findings | {{total_findings}} |

## Check Results

| Check | Findings |
|-------|----------|
{{#each checks}}
| {{name}} | {{count}} |
{{/each}}

## Type Breakdown

| Kind | Count |
|------|-------|
{{#each type_breakdown}}
| {{kind}} | {{count}} |
{{/each}}
"#;

const COMPLETENESS_REPORT_HTML: &str = r#"<html><head><title>Model Completeness Report</title>
<style>table{border-collapse:collapse;width:100%}th,td{border:1px solid #ddd;padding:8px;text-align:left}th{background:#f4f4f4}h1{color:#333}</style>
</head><body>
<h1>Model Completeness Report</h1>
<h2>Overview</h2>
<table><tbody>
<tr><td>Files</td><td>{{files}}</td></tr>
<tr><td>Elements</td><td>{{elements}}</td></tr>
<tr><td>Relationships</td><td>{{relationships}}</td></tr>
<tr><td>Completeness Score</td><td>{{completeness_score}}</td></tr>
<tr><td>Total Findings</td><td>{{total_findings}}</td></tr>
</tbody></table>
<h2>Check Results</h2>
<table><thead><tr><th>Check</th><th>Findings</th></tr></thead><tbody>
{{#each checks}}
<tr><td>{{name}}</td><td>{{count}}</td></tr>
{{/each}}
</tbody></table>
<h2>Type Breakdown</h2>
<table><thead><tr><th>Kind</th><th>Count</th></tr></thead><tbody>
{{#each type_breakdown}}
<tr><td>{{kind}}</td><td>{{count}}</td></tr>
{{/each}}
</tbody></table>
</body></html>"#;

const COMPLETENESS_REPORT_CSV: &str = r#"Metric,Value
Files,{{files}}
Elements,{{elements}}
Relationships,{{relationships}}
Completeness Score,{{completeness_score}}
Total Findings,{{total_findings}}

Check,Findings
{{#each checks}}
{{name}},{{count}}
{{/each}}

Kind,Count
{{#each type_breakdown}}
{{kind}},{{count}}
{{/each}}"#;

fn get_template(builtin: BuiltinTemplate, format: RenderFormat) -> &'static str {
    match (builtin, format) {
        (BuiltinTemplate::TraceabilityMatrix, RenderFormat::Markdown) => TRACEABILITY_MATRIX_MD,
        (BuiltinTemplate::TraceabilityMatrix, RenderFormat::Html) => TRACEABILITY_MATRIX_HTML,
        (BuiltinTemplate::TraceabilityMatrix, RenderFormat::Csv) => TRACEABILITY_MATRIX_CSV,
        (BuiltinTemplate::RequirementsTable, RenderFormat::Markdown) => REQUIREMENTS_TABLE_MD,
        (BuiltinTemplate::RequirementsTable, RenderFormat::Html) => REQUIREMENTS_TABLE_HTML,
        (BuiltinTemplate::RequirementsTable, RenderFormat::Csv) => REQUIREMENTS_TABLE_CSV,
        (BuiltinTemplate::CompletenessReport, RenderFormat::Markdown) => COMPLETENESS_REPORT_MD,
        (BuiltinTemplate::CompletenessReport, RenderFormat::Html) => COMPLETENESS_REPORT_HTML,
        (BuiltinTemplate::CompletenessReport, RenderFormat::Csv) => COMPLETENESS_REPORT_CSV,
    }
}

fn short_name(qualified: &str) -> &str {
    qualified.rsplit("::").next().unwrap_or(qualified)
}

fn short_path(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}

fn is_requirement(elem: &SysmlElement) -> bool {
    elem.kind.to_lowercase().contains("requirement")
}

fn build_satisfy_map(rels: &[SysmlRelationship]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for rel in rels {
        if rel.kind.eq_ignore_ascii_case("satisfy") {
            map.entry(rel.target.to_lowercase())
                .or_default()
                .push(short_name(&rel.source).to_string());
            let short = short_name(&rel.target).to_lowercase();
            if short != rel.target.to_lowercase() {
                map.entry(short)
                    .or_default()
                    .push(short_name(&rel.source).to_string());
            }
        }
    }
    map
}

fn build_verify_map(rels: &[SysmlRelationship]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for rel in rels {
        if rel.kind.eq_ignore_ascii_case("verify") {
            map.entry(rel.target.to_lowercase())
                .or_default()
                .push(short_name(&rel.source).to_string());
            let short = short_name(&rel.target).to_lowercase();
            if short != rel.target.to_lowercase() {
                map.entry(short)
                    .or_default()
                    .push(short_name(&rel.source).to_string());
            }
        }
    }
    map
}

fn build_traceability_context(graph: &SysmlGraph) -> TraceabilityContext {
    let satisfy_map = build_satisfy_map(graph.relationships());
    let verify_map = build_verify_map(graph.relationships());

    let mut rows = Vec::new();
    let mut satisfied_count = 0;
    let mut verified_count = 0;

    for elem in graph.elements() {
        if !is_requirement(elem) {
            continue;
        }

        let qname_lower = elem.qualified_name.to_lowercase();
        let short_lower = short_name(&elem.qualified_name).to_lowercase();

        let mut satisfied_by: Vec<String> = Vec::new();
        if let Some(v) = satisfy_map.get(&qname_lower) {
            satisfied_by.extend(v.iter().cloned());
        }
        if qname_lower != short_lower {
            if let Some(v) = satisfy_map.get(&short_lower) {
                for s in v {
                    if !satisfied_by.contains(s) {
                        satisfied_by.push(s.clone());
                    }
                }
            }
        }

        let mut verified_by: Vec<String> = Vec::new();
        if let Some(v) = verify_map.get(&qname_lower) {
            verified_by.extend(v.iter().cloned());
        }
        if qname_lower != short_lower {
            if let Some(v) = verify_map.get(&short_lower) {
                for s in v {
                    if !verified_by.contains(s) {
                        verified_by.push(s.clone());
                    }
                }
            }
        }

        let has_satisfy = !satisfied_by.is_empty();
        let has_verify = !verified_by.is_empty();

        if has_satisfy {
            satisfied_count += 1;
        }
        if has_verify {
            verified_count += 1;
        }

        let status = if has_satisfy && has_verify {
            "complete"
        } else if has_satisfy || has_verify {
            "partial"
        } else {
            "gap"
        };

        rows.push(TraceabilityRow {
            requirement: short_name(&elem.qualified_name).to_string(),
            kind: elem.kind.clone(),
            file: short_path(&elem.file_path),
            satisfied_by,
            verified_by,
            status: status.to_string(),
        });
    }

    let total = rows.len();
    let coverage_pct = if total > 0 {
        format!(
            "{:.0}",
            (satisfied_count.min(total) + verified_count.min(total)) as f64 / (2 * total) as f64
                * 100.0
        )
    } else {
        "100".to_string()
    };

    TraceabilityContext {
        rows,
        total_requirements: total,
        satisfied_count,
        verified_count,
        coverage_pct,
    }
}

fn build_requirements_context(graph: &SysmlGraph) -> RequirementsContext {
    let mut rows: Vec<RequirementRow> = graph
        .elements()
        .iter()
        .filter(|e| is_requirement(e))
        .map(|e| RequirementRow {
            name: short_name(&e.qualified_name).to_string(),
            kind: e.kind.clone(),
            file: short_path(&e.file_path),
            line: e.span.start_line,
            doc: e.doc.as_deref().unwrap_or("").to_string(),
            member_count: e.members.len(),
        })
        .collect();

    rows.sort_by(|a, b| a.name.cmp(&b.name));
    let total = rows.len();
    RequirementsContext { rows, total }
}

fn build_completeness_context(graph: &SysmlGraph) -> CompletenessContext {
    let all_checks = [
        (CheckType::OrphanRequirements, "Orphan Requirements"),
        (CheckType::UnverifiedRequirements, "Unverified Requirements"),
        (CheckType::MissingVerification, "Missing Verification"),
        (CheckType::UnconnectedPorts, "Unconnected Ports"),
        (CheckType::DanglingReferences, "Dangling References"),
    ];

    let mut checks = Vec::new();
    let mut total_findings = 0;
    let mut orphan_count = 0;
    let mut unverified_count = 0;

    for (ct, name) in &all_checks {
        let findings = graph.check(ct.clone());
        let count = findings.len();
        total_findings += count;
        match ct {
            CheckType::OrphanRequirements => orphan_count = count,
            CheckType::UnverifiedRequirements => unverified_count = count,
            _ => {}
        }
        checks.push(CheckSummary {
            name: name.to_string(),
            count,
        });
    }

    let total_requirements = graph
        .elements()
        .iter()
        .filter(|e| is_requirement(e))
        .count();

    let completeness_score = if total_requirements > 0 {
        let gap = (orphan_count + unverified_count).min(total_requirements);
        1.0 - (gap as f64 / total_requirements as f64)
    } else {
        1.0
    };

    let mut type_map: HashMap<&str, usize> = HashMap::new();
    for elem in graph.elements() {
        *type_map.entry(&elem.kind).or_insert(0) += 1;
    }
    let mut type_breakdown: Vec<TypeCount> = type_map
        .into_iter()
        .map(|(kind, count)| TypeCount {
            kind: kind.to_string(),
            count,
        })
        .collect();
    type_breakdown.sort_by(|a, b| b.count.cmp(&a.count));

    CompletenessContext {
        files: graph.file_count(),
        elements: graph.element_count(),
        relationships: graph.relationship_count(),
        completeness_score: format!("{:.3}", completeness_score),
        checks,
        total_findings,
        type_breakdown,
    }
}

pub fn render_builtin(
    graph: &SysmlGraph,
    template: BuiltinTemplate,
    format: RenderFormat,
) -> Result<String, RenderError> {
    let tmpl_str = get_template(template, format);
    let mut hbs = Handlebars::new();
    hbs.set_strict_mode(false);
    hbs.register_template_string("t", tmpl_str)
        .map_err(|e| RenderError::TemplateParse(Box::new(e)))?;

    match template {
        BuiltinTemplate::TraceabilityMatrix => {
            let ctx = build_traceability_context(graph);
            Ok(hbs.render("t", &ctx)?)
        }
        BuiltinTemplate::RequirementsTable => {
            let ctx = build_requirements_context(graph);
            Ok(hbs.render("t", &ctx)?)
        }
        BuiltinTemplate::CompletenessReport => {
            let ctx = build_completeness_context(graph);
            Ok(hbs.render("t", &ctx)?)
        }
    }
}

pub fn render_custom(graph: &SysmlGraph, template_path: &Path) -> Result<String, RenderError> {
    let tmpl_str = std::fs::read_to_string(template_path)?;
    let mut hbs = Handlebars::new();
    hbs.set_strict_mode(false);
    hbs.register_template_string("t", &tmpl_str)
        .map_err(|e| RenderError::TemplateParse(Box::new(e)))?;

    let ctx = build_full_context(graph);
    Ok(hbs.render("t", &ctx)?)
}

#[derive(Serialize)]
struct FullContext {
    files: usize,
    elements: usize,
    relationships: usize,
    traceability: TraceabilityContext,
    requirements: RequirementsContext,
    completeness: CompletenessContext,
}

fn build_full_context(graph: &SysmlGraph) -> FullContext {
    FullContext {
        files: graph.file_count(),
        elements: graph.element_count(),
        relationships: graph.relationship_count(),
        traceability: build_traceability_context(graph),
        requirements: build_requirements_context(graph),
        completeness: build_completeness_context(graph),
    }
}

pub fn parse_builtin_template(name: &str) -> Option<BuiltinTemplate> {
    match name.to_lowercase().replace('-', "_").as_str() {
        "traceability_matrix" | "traceability" => Some(BuiltinTemplate::TraceabilityMatrix),
        "requirements_table" | "requirements" => Some(BuiltinTemplate::RequirementsTable),
        "completeness_report" | "completeness" => Some(BuiltinTemplate::CompletenessReport),
        _ => None,
    }
}

pub fn parse_render_format(name: &str) -> Option<RenderFormat> {
    match name.to_lowercase().as_str() {
        "markdown" | "md" => Some(RenderFormat::Markdown),
        "html" => Some(RenderFormat::Html),
        "csv" => Some(RenderFormat::Csv),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SysmlParser;
    use crate::core_traits::Parser as NomographParser;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/eve")
    }

    fn walkdir(dir: PathBuf) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(walkdir(path));
                } else {
                    files.push(path);
                }
            }
        }
        files
    }

    fn build_eve_graph() -> SysmlGraph {
        let parser = SysmlParser::new();
        let mut results = Vec::new();
        for entry in walkdir(fixture_dir()) {
            if entry.extension().and_then(|e| e.to_str()) == Some("sysml") {
                let source = std::fs::read_to_string(&entry).expect("read fixture");
                let result = parser.parse(&source, &entry).expect("parse fixture");
                results.push(result);
            }
        }
        let mut graph = SysmlGraph::new();
        graph.index(results).expect("index");
        graph
    }

    #[test]
    fn test_render_traceability_matrix_md() {
        let graph = build_eve_graph();
        let output = render_builtin(
            &graph,
            BuiltinTemplate::TraceabilityMatrix,
            RenderFormat::Markdown,
        )
        .expect("render should succeed");
        assert!(output.contains("Traceability Matrix"));
        assert!(output.contains("Requirement"));
        assert!(output.contains("Summary"));
    }

    #[test]
    fn test_render_traceability_matrix_html() {
        let graph = build_eve_graph();
        let output = render_builtin(
            &graph,
            BuiltinTemplate::TraceabilityMatrix,
            RenderFormat::Html,
        )
        .expect("render should succeed");
        assert!(output.contains("<html>"));
        assert!(output.contains("Traceability Matrix"));
        assert!(output.contains("<table>"));
    }

    #[test]
    fn test_render_traceability_matrix_csv() {
        let graph = build_eve_graph();
        let output = render_builtin(
            &graph,
            BuiltinTemplate::TraceabilityMatrix,
            RenderFormat::Csv,
        )
        .expect("render should succeed");
        assert!(output.contains("Requirement,Kind,Satisfied By,Verified By,Status"));
    }

    #[test]
    fn test_render_requirements_table_md() {
        let graph = build_eve_graph();
        let output = render_builtin(
            &graph,
            BuiltinTemplate::RequirementsTable,
            RenderFormat::Markdown,
        )
        .expect("render should succeed");
        assert!(output.contains("Requirements Table"));
        assert!(output.contains("Total"));
    }

    #[test]
    fn test_render_completeness_report_md() {
        let graph = build_eve_graph();
        let output = render_builtin(
            &graph,
            BuiltinTemplate::CompletenessReport,
            RenderFormat::Markdown,
        )
        .expect("render should succeed");
        assert!(output.contains("Model Completeness Report"));
        assert!(output.contains("Files"));
        assert!(output.contains("Completeness Score"));
        assert!(output.contains("Orphan Requirements"));
    }

    #[test]
    fn test_render_completeness_report_html() {
        let graph = build_eve_graph();
        let output = render_builtin(
            &graph,
            BuiltinTemplate::CompletenessReport,
            RenderFormat::Html,
        )
        .expect("render should succeed");
        assert!(output.contains("<html>"));
        assert!(output.contains("Model Completeness Report"));
    }

    #[test]
    fn test_parse_builtin_template() {
        assert_eq!(
            parse_builtin_template("traceability-matrix"),
            Some(BuiltinTemplate::TraceabilityMatrix)
        );
        assert_eq!(
            parse_builtin_template("requirements-table"),
            Some(BuiltinTemplate::RequirementsTable)
        );
        assert_eq!(
            parse_builtin_template("completeness-report"),
            Some(BuiltinTemplate::CompletenessReport)
        );
        assert_eq!(
            parse_builtin_template("traceability"),
            Some(BuiltinTemplate::TraceabilityMatrix)
        );
        assert!(parse_builtin_template("unknown").is_none());
    }

    #[test]
    fn test_parse_render_format() {
        assert_eq!(
            parse_render_format("markdown"),
            Some(RenderFormat::Markdown)
        );
        assert_eq!(parse_render_format("md"), Some(RenderFormat::Markdown));
        assert_eq!(parse_render_format("html"), Some(RenderFormat::Html));
        assert_eq!(parse_render_format("csv"), Some(RenderFormat::Csv));
        assert!(parse_render_format("xml").is_none());
    }
}
