use std::collections::HashMap;

use crate::element::SysmlElement;
use crate::graph::SysmlGraph;
use crate::relationship::SysmlRelationship;
use crate::core_traits::KnowledgeGraph;
use crate::core_types::Finding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetamodelCheck {
    SatisfyTargetMustBeRequirement,
    VerifyTargetMustBeRequirement,
    AllocateSourceLogicalTargetPhysical,
    PortsMustHaveType,
    BindingConnectorCompatibility,
}

fn build_element_map(elements: &[SysmlElement]) -> HashMap<String, &SysmlElement> {
    let mut map = HashMap::new();
    for elem in elements {
        map.insert(elem.qualified_name.to_lowercase(), elem);
        let short = elem
            .qualified_name
            .rsplit("::")
            .next()
            .unwrap_or(&elem.qualified_name)
            .to_lowercase();
        map.entry(short).or_insert(elem);
    }
    map
}

fn resolve_element<'a>(
    name: &str,
    elem_map: &HashMap<String, &'a SysmlElement>,
) -> Option<&'a SysmlElement> {
    let lower = name.to_lowercase();
    if let Some(e) = elem_map.get(&lower) {
        return Some(e);
    }
    let short = name.rsplit("::").next().unwrap_or(name).to_lowercase();
    elem_map.get(&short).copied()
}

fn is_requirement(elem: &SysmlElement) -> bool {
    elem.kind.to_lowercase().contains("requirement")
}

fn is_port(elem: &SysmlElement) -> bool {
    elem.kind.to_lowercase().contains("port")
}

fn is_logical_kind(kind: &str) -> bool {
    let k = kind.to_lowercase();
    k.contains("requirement")
        || k.contains("use_case")
        || k.contains("action")
        || k.contains("state")
}

fn is_physical_kind(kind: &str) -> bool {
    let k = kind.to_lowercase();
    k.contains("part") || k.contains("item") || k.contains("port")
}

fn check_satisfy_targets(
    rels: &[SysmlRelationship],
    elem_map: &HashMap<String, &SysmlElement>,
) -> Vec<Finding> {
    rels.iter()
        .filter(|r| r.kind.eq_ignore_ascii_case("satisfy"))
        .filter_map(|r| {
            let target = resolve_element(&r.target, elem_map);
            match target {
                Some(e) if is_requirement(e) => None,
                Some(e) => Some(Finding {
                    check_type: crate::core_types::CheckType::DanglingReferences,
                    element: r.source.clone(),
                    message: format!(
                        "satisfy target '{}' is {} (expected requirement)",
                        r.target, e.kind
                    ),
                    file_path: r.file_path.clone(),
                    span: r.span.clone(),
                }),
                None => None,
            }
        })
        .collect()
}

fn check_verify_targets(
    rels: &[SysmlRelationship],
    elem_map: &HashMap<String, &SysmlElement>,
) -> Vec<Finding> {
    rels.iter()
        .filter(|r| r.kind.eq_ignore_ascii_case("verify"))
        .filter_map(|r| {
            let target = resolve_element(&r.target, elem_map);
            match target {
                Some(e) if is_requirement(e) => None,
                Some(e) => Some(Finding {
                    check_type: crate::core_types::CheckType::DanglingReferences,
                    element: r.source.clone(),
                    message: format!(
                        "verify target '{}' is {} (expected requirement)",
                        r.target, e.kind
                    ),
                    file_path: r.file_path.clone(),
                    span: r.span.clone(),
                }),
                None => None,
            }
        })
        .collect()
}

fn check_allocate_layers(
    rels: &[SysmlRelationship],
    elem_map: &HashMap<String, &SysmlElement>,
) -> Vec<Finding> {
    rels.iter()
        .filter(|r| r.kind.eq_ignore_ascii_case("allocate"))
        .filter_map(|r| {
            let source = resolve_element(&r.source, elem_map);
            let target = resolve_element(&r.target, elem_map);
            match (source, target) {
                (Some(s), Some(t)) => {
                    let mut findings = Vec::new();
                    if !is_logical_kind(&s.kind) {
                        findings.push(Finding {
                            check_type: crate::core_types::CheckType::DanglingReferences,
                            element: r.source.clone(),
                            message: format!(
                                "allocate source '{}' is {} (expected logical element)",
                                r.source, s.kind
                            ),
                            file_path: r.file_path.clone(),
                            span: r.span.clone(),
                        });
                    }
                    if !is_physical_kind(&t.kind) {
                        findings.push(Finding {
                            check_type: crate::core_types::CheckType::DanglingReferences,
                            element: r.target.clone(),
                            message: format!(
                                "allocate target '{}' is {} (expected physical element)",
                                r.target, t.kind
                            ),
                            file_path: r.file_path.clone(),
                            span: r.span.clone(),
                        });
                    }
                    if findings.is_empty() {
                        None
                    } else {
                        Some(findings)
                    }
                }
                _ => None,
            }
        })
        .flatten()
        .collect()
}

fn check_ports_have_type(elements: &[SysmlElement], rels: &[SysmlRelationship]) -> Vec<Finding> {
    let typed_sources: std::collections::HashSet<String> = rels
        .iter()
        .filter(|r| r.kind.eq_ignore_ascii_case("typedby"))
        .flat_map(|r| {
            let short = r
                .source
                .rsplit("::")
                .next()
                .unwrap_or(&r.source)
                .to_lowercase();
            vec![r.source.to_lowercase(), short]
        })
        .collect();

    elements
        .iter()
        .filter(|e| is_port(e))
        .filter(|e| {
            let qname = e.qualified_name.to_lowercase();
            let short = e
                .qualified_name
                .rsplit("::")
                .next()
                .unwrap_or(&e.qualified_name)
                .to_lowercase();
            !typed_sources.contains(&qname) && !typed_sources.contains(&short)
        })
        .map(|e| Finding {
            check_type: crate::core_types::CheckType::DanglingReferences,
            element: e.qualified_name.clone(),
            message: "port has no TypedBy relationship (missing type definition)".to_string(),
            file_path: e.file_path.clone(),
            span: e.span.clone(),
        })
        .collect()
}

fn check_binding_connector_compatibility(
    rels: &[SysmlRelationship],
    _elem_map: &HashMap<String, &SysmlElement>,
) -> Vec<Finding> {
    let typed_by: HashMap<String, String> = rels
        .iter()
        .filter(|r| r.kind.eq_ignore_ascii_case("typedby"))
        .map(|r| (r.source.to_lowercase(), r.target.to_lowercase()))
        .collect();

    rels.iter()
        .filter(|r| {
            let k = r.kind.to_lowercase();
            k == "connect" || k == "bind" || k == "binding"
        })
        .filter_map(|r| {
            let src_type = typed_by.get(&r.source.to_lowercase());
            let tgt_type = typed_by.get(&r.target.to_lowercase());

            match (src_type, tgt_type) {
                (Some(st), Some(tt)) if st != tt => Some(Finding {
                    check_type: crate::core_types::CheckType::DanglingReferences,
                    element: r.source.clone(),
                    message: format!(
                        "binding connector connects incompatible types: '{}' ({}) to '{}' ({})",
                        r.source, st, r.target, tt
                    ),
                    file_path: r.file_path.clone(),
                    span: r.span.clone(),
                }),
                _ => None,
            }
        })
        .collect()
}

pub fn run_metamodel_checks(graph: &SysmlGraph) -> Vec<Finding> {
    let elem_map = build_element_map(graph.elements());
    let rels = graph.relationships();
    let elements = graph.elements();

    let mut findings = Vec::new();
    findings.extend(check_satisfy_targets(rels, &elem_map));
    findings.extend(check_verify_targets(rels, &elem_map));
    findings.extend(check_allocate_layers(rels, &elem_map));
    findings.extend(check_ports_have_type(elements, rels));
    findings.extend(check_binding_connector_compatibility(rels, &elem_map));
    findings
}

pub fn run_single_metamodel_check(graph: &SysmlGraph, check: &MetamodelCheck) -> Vec<Finding> {
    let elem_map = build_element_map(graph.elements());
    let rels = graph.relationships();
    let elements = graph.elements();

    match check {
        MetamodelCheck::SatisfyTargetMustBeRequirement => check_satisfy_targets(rels, &elem_map),
        MetamodelCheck::VerifyTargetMustBeRequirement => check_verify_targets(rels, &elem_map),
        MetamodelCheck::AllocateSourceLogicalTargetPhysical => {
            check_allocate_layers(rels, &elem_map)
        }
        MetamodelCheck::PortsMustHaveType => check_ports_have_type(elements, rels),
        MetamodelCheck::BindingConnectorCompatibility => {
            check_binding_connector_compatibility(rels, &elem_map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SysmlParser;
    use crate::core_traits::Parser as NomographParser;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/eve")
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
    fn test_metamodel_checks_run() {
        let graph = build_eve_graph();
        let findings = run_metamodel_checks(&graph);
        assert!(
            findings.iter().all(|f| !f.message.is_empty()),
            "all findings should have messages"
        );
    }

    #[test]
    fn test_ports_have_type_check() {
        let graph = build_eve_graph();
        let findings = run_single_metamodel_check(&graph, &MetamodelCheck::PortsMustHaveType);
        for f in &findings {
            assert!(
                f.message.contains("port has no TypedBy"),
                "finding should be about missing port type"
            );
        }
    }

    #[test]
    fn test_satisfy_target_check() {
        let graph = build_eve_graph();
        let findings =
            run_single_metamodel_check(&graph, &MetamodelCheck::SatisfyTargetMustBeRequirement);
        for f in &findings {
            assert!(
                f.message.contains("satisfy target"),
                "finding should be about satisfy target"
            );
        }
    }

    #[test]
    fn test_verify_target_check() {
        let graph = build_eve_graph();
        let findings =
            run_single_metamodel_check(&graph, &MetamodelCheck::VerifyTargetMustBeRequirement);
        for f in &findings {
            assert!(
                f.message.contains("verify target"),
                "finding should be about verify target"
            );
        }
    }
}
