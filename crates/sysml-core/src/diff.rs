use std::collections::{HashMap, HashSet};

use serde::Serialize;

use nomograph_core::traits::KnowledgeGraph;

use crate::element::SysmlElement;
use crate::graph::SysmlGraph;
use crate::relationship::SysmlRelationship;

#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub elements_added: Vec<ElementChange>,
    pub elements_removed: Vec<ElementChange>,
    pub elements_modified: Vec<ElementModification>,
    pub relationships_added: Vec<RelationshipChange>,
    pub relationships_removed: Vec<RelationshipChange>,
    pub summary: DiffSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElementChange {
    pub qualified_name: String,
    pub kind: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElementModification {
    pub qualified_name: String,
    pub kind: String,
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationshipChange {
    pub source: String,
    pub target: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffSummary {
    pub elements_added: usize,
    pub elements_removed: usize,
    pub elements_modified: usize,
    pub relationships_added: usize,
    pub relationships_removed: usize,
    pub total_changes: usize,
}

fn element_key(e: &SysmlElement) -> String {
    e.qualified_name.to_lowercase()
}

fn rel_key(r: &SysmlRelationship) -> String {
    format!(
        "{}|{}|{}",
        r.source.to_lowercase(),
        r.kind.to_lowercase(),
        r.target.to_lowercase()
    )
}

pub fn diff_graphs(base: &SysmlGraph, head: &SysmlGraph) -> DiffResult {
    let base_elements: HashMap<String, &SysmlElement> = base
        .elements()
        .iter()
        .map(|e| (element_key(e), e))
        .collect();
    let head_elements: HashMap<String, &SysmlElement> = head
        .elements()
        .iter()
        .map(|e| (element_key(e), e))
        .collect();

    let base_keys: HashSet<&String> = base_elements.keys().collect();
    let head_keys: HashSet<&String> = head_elements.keys().collect();

    let mut elements_added = Vec::new();
    for key in head_keys.difference(&base_keys) {
        if let Some(e) = head_elements.get(*key) {
            elements_added.push(ElementChange {
                qualified_name: e.qualified_name.clone(),
                kind: e.kind.clone(),
                file_path: e.file_path.to_string_lossy().to_string(),
            });
        }
    }
    elements_added.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

    let mut elements_removed = Vec::new();
    for key in base_keys.difference(&head_keys) {
        if let Some(e) = base_elements.get(*key) {
            elements_removed.push(ElementChange {
                qualified_name: e.qualified_name.clone(),
                kind: e.kind.clone(),
                file_path: e.file_path.to_string_lossy().to_string(),
            });
        }
    }
    elements_removed.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

    let mut elements_modified = Vec::new();
    for key in base_keys.intersection(&head_keys) {
        if let (Some(base_e), Some(head_e)) = (base_elements.get(*key), head_elements.get(*key)) {
            let mut changes = Vec::new();
            if base_e.kind != head_e.kind {
                changes.push(format!("kind: {} -> {}", base_e.kind, head_e.kind));
            }
            if base_e.doc != head_e.doc {
                changes.push("doc changed".to_string());
            }
            if base_e.layer != head_e.layer {
                changes.push(format!("layer: {:?} -> {:?}", base_e.layer, head_e.layer));
            }
            if base_e.members != head_e.members {
                let base_set: HashSet<&String> = base_e.members.iter().collect();
                let head_set: HashSet<&String> = head_e.members.iter().collect();
                let added: Vec<_> = head_set.difference(&base_set).collect();
                let removed: Vec<_> = base_set.difference(&head_set).collect();
                if !added.is_empty() {
                    changes.push(format!("members added: {}", added.len()));
                }
                if !removed.is_empty() {
                    changes.push(format!("members removed: {}", removed.len()));
                }
            }
            if !changes.is_empty() {
                elements_modified.push(ElementModification {
                    qualified_name: head_e.qualified_name.clone(),
                    kind: head_e.kind.clone(),
                    changes,
                });
            }
        }
    }
    elements_modified.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

    let base_rels: HashSet<String> = base.relationships().iter().map(rel_key).collect();
    let head_rels: HashSet<String> = head.relationships().iter().map(rel_key).collect();
    let head_rel_map: HashMap<String, &SysmlRelationship> = head
        .relationships()
        .iter()
        .map(|r| (rel_key(r), r))
        .collect();
    let base_rel_map: HashMap<String, &SysmlRelationship> = base
        .relationships()
        .iter()
        .map(|r| (rel_key(r), r))
        .collect();

    let mut relationships_added: Vec<RelationshipChange> = head_rels
        .difference(&base_rels)
        .filter_map(|k| head_rel_map.get(k))
        .map(|r| RelationshipChange {
            source: r.source.clone(),
            target: r.target.clone(),
            kind: r.kind.clone(),
        })
        .collect();
    relationships_added.sort_by(|a, b| a.source.cmp(&b.source).then(a.kind.cmp(&b.kind)));

    let mut relationships_removed: Vec<RelationshipChange> = base_rels
        .difference(&head_rels)
        .filter_map(|k| base_rel_map.get(k))
        .map(|r| RelationshipChange {
            source: r.source.clone(),
            target: r.target.clone(),
            kind: r.kind.clone(),
        })
        .collect();
    relationships_removed.sort_by(|a, b| a.source.cmp(&b.source).then(a.kind.cmp(&b.kind)));

    let total = elements_added.len()
        + elements_removed.len()
        + elements_modified.len()
        + relationships_added.len()
        + relationships_removed.len();

    let summary = DiffSummary {
        elements_added: elements_added.len(),
        elements_removed: elements_removed.len(),
        elements_modified: elements_modified.len(),
        relationships_added: relationships_added.len(),
        relationships_removed: relationships_removed.len(),
        total_changes: total,
    };

    DiffResult {
        elements_added,
        elements_removed,
        elements_modified,
        relationships_added,
        relationships_removed,
        summary,
    }
}

pub fn format_compact(result: &DiffResult) -> Vec<String> {
    let mut lines = Vec::new();
    for e in &result.elements_added {
        lines.push(format!("+ {} ({})", e.qualified_name, e.kind));
    }
    for e in &result.elements_removed {
        lines.push(format!("- {} ({})", e.qualified_name, e.kind));
    }
    for e in &result.elements_modified {
        lines.push(format!("~ {} [{}]", e.qualified_name, e.changes.join(", ")));
    }
    for r in &result.relationships_added {
        lines.push(format!("+ {} -> {} -> {}", r.source, r.kind, r.target));
    }
    for r in &result.relationships_removed {
        lines.push(format!("- {} -> {} -> {}", r.source, r.kind, r.target));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::SysmlGraph;
    use nomograph_core::types::ParseResult;
    use std::path::PathBuf;

    fn make_element(name: &str, kind: &str) -> SysmlElement {
        SysmlElement {
            qualified_name: name.to_string(),
            kind: kind.to_string(),
            file_path: PathBuf::from("test.sysml"),
            span: nomograph_core::types::Span {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            },
            doc: None,
            attributes: Vec::new(),
            members: Vec::new(),
            layer: None,
        }
    }

    fn make_rel(source: &str, kind: &str, target: &str) -> SysmlRelationship {
        SysmlRelationship {
            source: source.to_string(),
            target: target.to_string(),
            kind: kind.to_string(),
            file_path: PathBuf::from("test.sysml"),
            span: nomograph_core::types::Span {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            },
        }
    }

    fn build_graph(
        elements: Vec<SysmlElement>,
        relationships: Vec<SysmlRelationship>,
    ) -> SysmlGraph {
        let mut graph = SysmlGraph::new();
        let result = ParseResult {
            elements,
            relationships,
            diagnostics: Vec::new(),
        };
        graph.index(vec![result]).unwrap();
        graph
    }

    #[test]
    fn test_diff_no_changes() {
        let base = build_graph(vec![make_element("A", "part_usage")], vec![]);
        let head = build_graph(vec![make_element("A", "part_usage")], vec![]);
        let result = diff_graphs(&base, &head);
        assert_eq!(result.summary.total_changes, 0);
    }

    #[test]
    fn test_diff_element_added() {
        let base = build_graph(vec![make_element("A", "part_usage")], vec![]);
        let head = build_graph(
            vec![
                make_element("A", "part_usage"),
                make_element("B", "requirement_usage"),
            ],
            vec![],
        );
        let result = diff_graphs(&base, &head);
        assert_eq!(result.summary.elements_added, 1);
        assert_eq!(result.elements_added[0].qualified_name, "B");
    }

    #[test]
    fn test_diff_element_removed() {
        let base = build_graph(
            vec![
                make_element("A", "part_usage"),
                make_element("B", "requirement_usage"),
            ],
            vec![],
        );
        let head = build_graph(vec![make_element("A", "part_usage")], vec![]);
        let result = diff_graphs(&base, &head);
        assert_eq!(result.summary.elements_removed, 1);
        assert_eq!(result.elements_removed[0].qualified_name, "B");
    }

    #[test]
    fn test_diff_element_modified() {
        let mut e = make_element("A", "part_usage");
        e.doc = Some("old doc".to_string());
        let base = build_graph(vec![e], vec![]);

        let mut e2 = make_element("A", "part_usage");
        e2.doc = Some("new doc".to_string());
        let head = build_graph(vec![e2], vec![]);

        let result = diff_graphs(&base, &head);
        assert_eq!(result.summary.elements_modified, 1);
        assert!(result.elements_modified[0]
            .changes
            .contains(&"doc changed".to_string()));
    }

    #[test]
    fn test_diff_relationship_added() {
        let base = build_graph(vec![make_element("A", "part_usage")], vec![]);
        let head = build_graph(
            vec![make_element("A", "part_usage")],
            vec![make_rel("A", "Satisfy", "B")],
        );
        let result = diff_graphs(&base, &head);
        assert_eq!(result.summary.relationships_added, 1);
    }

    #[test]
    fn test_diff_relationship_removed() {
        let base = build_graph(
            vec![make_element("A", "part_usage")],
            vec![make_rel("A", "Satisfy", "B")],
        );
        let head = build_graph(vec![make_element("A", "part_usage")], vec![]);
        let result = diff_graphs(&base, &head);
        assert_eq!(result.summary.relationships_removed, 1);
    }

    #[test]
    fn test_compact_format() {
        let base = build_graph(vec![make_element("A", "part_usage")], vec![]);
        let head = build_graph(
            vec![
                make_element("A", "part_usage"),
                make_element("B", "requirement_usage"),
            ],
            vec![make_rel("A", "Satisfy", "B")],
        );
        let result = diff_graphs(&base, &head);
        let lines = format_compact(&result);
        assert!(lines.iter().any(|l| l.starts_with("+ B")));
        assert!(lines.iter().any(|l| l.contains("Satisfy")));
    }
}
