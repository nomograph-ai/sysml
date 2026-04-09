use std::path::PathBuf;

use tree_sitter::Node;

use crate::core_types::{Diagnostic, Severity, Span};

use crate::element::SysmlElement;
use crate::relationship::SysmlRelationship;

pub fn is_element_node(kind: &str) -> bool {
    kind.ends_with("_definition") || kind.ends_with("_usage") || kind == "library_package"
}

fn is_body_node(kind: &str) -> bool {
    matches!(
        kind,
        "package_body"
            | "structural_body"
            | "usage_body"
            | "action_body"
            | "constraint_body"
            | "enumeration_body"
            | "requirement_body"
            | "state_body"
    )
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(s)
}

fn extract_element_name(node: &Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "identification" {
                return find_name_in_identification(&child, source);
            }
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "usage_declaration" {
                for j in 0..child.child_count() {
                    if let Some(gc) = child.child(j) {
                        if gc.kind() == "identification" {
                            return find_name_in_identification(&gc, source);
                        }
                    }
                }
                if let Some(rel_part) = find_child_by_kind(&child, "relationship_part") {
                    if let Some(redef) = find_child_by_kind(&rel_part, "redefinition_part") {
                        if let Some(fc) = find_child_by_kind(&redef, "feature_chain") {
                            return extract_feature_chain_text(&fc, source);
                        }
                    }
                }
            }
        }
    }

    if let Some(fc) = find_child_by_kind(node, "feature_chain") {
        return extract_feature_chain_text(&fc, source);
    }

    None
}

fn find_name_in_identification(node: &Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "name" {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| strip_quotes(s).to_string());
            }
        }
    }
    None
}

fn extract_typed_by(node: &Node, source: &str) -> Option<String> {
    let decl = find_child_by_kind(node, "usage_declaration")?;
    let typing = find_child_by_kind(&decl, "typing_part")?;
    let qname = find_child_by_kind(&typing, "qualified_name")?;
    extract_qualified_name_text(&qname, source)
}

fn extract_qualified_name_text(node: &Node, source: &str) -> Option<String> {
    let mut parts = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "name" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    parts.push(strip_quotes(text).to_string());
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("::"))
    }
}

fn find_child_by_kind<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

fn extract_doc_comment(body_node: &Node, source: &str) -> Option<String> {
    for i in 0..body_node.child_count() {
        if let Some(child) = body_node.child(i) {
            if child.kind() == "documentation" {
                if let Some(comment_body) = find_child_by_kind(&child, "block_comment_body") {
                    if let Ok(text) = comment_body.utf8_text(source.as_bytes()) {
                        return Some(clean_doc_comment(text));
                    }
                }
            }
        }
    }
    None
}

fn clean_doc_comment(text: &str) -> String {
    let trimmed = text
        .strip_prefix("/*")
        .unwrap_or(text)
        .strip_suffix("*/")
        .unwrap_or(text);
    trimmed
        .lines()
        .map(|line| {
            let stripped = line.trim();
            stripped
                .strip_prefix("* ")
                .unwrap_or(stripped.strip_prefix('*').unwrap_or(stripped))
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn find_body_child<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if is_body_node(child.kind()) {
                return Some(child);
            }
        }
    }
    None
}

fn extract_feature_chain_text(node: &Node, source: &str) -> Option<String> {
    let mut parts = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "name" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    parts.push(strip_quotes(text).to_string());
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

fn find_children_by_kind<'a>(node: &Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut result = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                result.push(child);
            }
        }
    }
    result
}

fn extract_binding_value_text(node: &Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.is_named() {
                if let Some(fce) = find_child_by_kind(&child, "qualified_name") {
                    return extract_qualified_name_text(&fce, source);
                }
                if let Some(fc) = find_child_by_kind(&child, "feature_chain") {
                    return extract_feature_chain_text(&fc, source);
                }
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.trim().to_string());
            }
        }
    }
    None
}

fn extract_reference_text(node: &Node, source: &str) -> Option<String> {
    if let Some(qname) = find_child_by_kind(node, "qualified_name") {
        return extract_qualified_name_text(&qname, source);
    }
    if let Some(fc) = find_child_by_kind(node, "feature_chain") {
        return extract_feature_chain_text(&fc, source);
    }
    if let Some(name) = find_child_by_kind(node, "name") {
        return name
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| strip_quotes(s).to_string());
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RelationshipKind {
    Satisfy,
    Verify,
    Import,
    Specialize,
    Allocate,
    Connect,
    Bind,
    Flow,
    Stream,
    Dependency,
    Redefine,
    Expose,
    Perform,
    Exhibit,
    Include,
    Succession,
    Transition,
    Send,
    Accept,
    Require,
    Assume,
    Assert,
    Assign,
    Subject,
    Render,
    Frame,
    Message,
    TypedBy,
    Member,
}

impl RelationshipKind {
    #[cfg(test)]
    pub(crate) const ALL: &[RelationshipKind] = &[
        Self::Satisfy,
        Self::Verify,
        Self::Import,
        Self::Specialize,
        Self::Allocate,
        Self::Connect,
        Self::Bind,
        Self::Flow,
        Self::Stream,
        Self::Dependency,
        Self::Redefine,
        Self::Expose,
        Self::Perform,
        Self::Exhibit,
        Self::Include,
        Self::Succession,
        Self::Transition,
        Self::Send,
        Self::Accept,
        Self::Require,
        Self::Assume,
        Self::Assert,
        Self::Assign,
        Self::Subject,
        Self::Render,
        Self::Frame,
        Self::Message,
        Self::TypedBy,
        Self::Member,
    ];
}

impl std::fmt::Display for RelationshipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Satisfy => write!(f, "Satisfy"),
            Self::Verify => write!(f, "Verify"),
            Self::Import => write!(f, "Import"),
            Self::Specialize => write!(f, "Specialize"),
            Self::Allocate => write!(f, "Allocate"),
            Self::Connect => write!(f, "Connect"),
            Self::Bind => write!(f, "Bind"),
            Self::Flow => write!(f, "Flow"),
            Self::Stream => write!(f, "Stream"),
            Self::Dependency => write!(f, "Dependency"),
            Self::Redefine => write!(f, "Redefine"),
            Self::Expose => write!(f, "Expose"),
            Self::Perform => write!(f, "Perform"),
            Self::Exhibit => write!(f, "Exhibit"),
            Self::Include => write!(f, "Include"),
            Self::Succession => write!(f, "Succession"),
            Self::Transition => write!(f, "Transition"),
            Self::Send => write!(f, "Send"),
            Self::Accept => write!(f, "Accept"),
            Self::Require => write!(f, "Require"),
            Self::Assume => write!(f, "Assume"),
            Self::Assert => write!(f, "Assert"),
            Self::Assign => write!(f, "Assign"),
            Self::Subject => write!(f, "Subject"),
            Self::Render => write!(f, "Render"),
            Self::Frame => write!(f, "Frame"),
            Self::Message => write!(f, "Message"),
            Self::TypedBy => write!(f, "TypedBy"),
            Self::Member => write!(f, "Member"),
        }
    }
}

pub(crate) const RELATIONSHIP_DISPATCH: &[(&str, RelationshipKind)] = &[
    ("satisfy_statement", RelationshipKind::Satisfy),
    ("verify_statement", RelationshipKind::Verify),
    ("import_statement", RelationshipKind::Import),
    ("definition_specialization", RelationshipKind::Specialize),
    ("allocate_statement", RelationshipKind::Allocate),
    ("connect_statement", RelationshipKind::Connect),
    ("bind_statement", RelationshipKind::Bind),
    ("flow_statement", RelationshipKind::Flow),
    ("flow_usage", RelationshipKind::Flow),
    ("stream_statement", RelationshipKind::Stream),
    ("dependency", RelationshipKind::Dependency),
    ("perform_statement", RelationshipKind::Perform),
    ("exhibit_usage", RelationshipKind::Exhibit),
    ("include_statement", RelationshipKind::Include),
    ("expose_statement", RelationshipKind::Expose),
    ("then_succession", RelationshipKind::Succession),
    ("succession_statement", RelationshipKind::Succession),
    ("first_statement", RelationshipKind::Succession),
    ("message_statement", RelationshipKind::Message),
    ("redefines_statement", RelationshipKind::Redefine),
    ("specialization_statement", RelationshipKind::Specialize),
    ("transition_statement", RelationshipKind::Transition),
    ("send_statement", RelationshipKind::Send),
    ("accept_then_statement", RelationshipKind::Accept),
    ("require_statement", RelationshipKind::Require),
    ("assume_statement", RelationshipKind::Assume),
    ("assert_statement", RelationshipKind::Assert),
    ("assign_statement", RelationshipKind::Assign),
    ("subject_statement", RelationshipKind::Subject),
    ("render_statement", RelationshipKind::Render),
    ("frame_statement", RelationshipKind::Frame),
];

fn dispatch_relationship_kind(node_kind: &str) -> Option<RelationshipKind> {
    RELATIONSHIP_DISPATCH
        .iter()
        .find(|(k, _)| *k == node_kind)
        .map(|(_, v)| *v)
}

fn node_span(node: &Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start_line: start.row as u32,
        start_col: start.column as u32,
        end_line: end.row as u32,
        end_col: end.column as u32,
    }
}

pub struct Walker<'a> {
    source: &'a str,
    file_path: PathBuf,
    name_stack: Vec<String>,
    pub elements: Vec<SysmlElement>,
    pub relationships: Vec<SysmlRelationship>,
    anon_counter: usize,
}

impl<'a> Walker<'a> {
    pub fn new(source: &'a str, file_path: PathBuf) -> Self {
        Self {
            source,
            file_path,
            name_stack: Vec::new(),
            elements: Vec::new(),
            relationships: Vec::new(),
            anon_counter: 0,
        }
    }

    fn qualified_name(&self) -> String {
        self.name_stack.join("::")
    }

    pub fn walk_root(&mut self, root: Node<'a>) {
        for i in 0..root.child_count() {
            if let Some(child) = root.child(i) {
                if child.is_named() {
                    self.walk_node(child);
                }
            }
        }
    }

    fn walk_node(&mut self, node: Node<'a>) {
        let kind = node.kind();
        if !is_element_node(kind) {
            return;
        }

        let name = extract_element_name(&node, self.source);
        if name.is_none() {
            return;
        }

        let display_name = name.clone().unwrap_or_else(|| {
            self.anon_counter += 1;
            format!("<anonymous_{}>", self.anon_counter)
        });

        self.name_stack.push(display_name);
        let qname = self.qualified_name();
        let span = node_span(&node);

        let body = find_body_child(&node);
        let doc = body
            .as_ref()
            .and_then(|b| extract_doc_comment(b, self.source));
        let typed_by = extract_typed_by(&node, self.source);
        let members = body
            .as_ref()
            .map(|b| self.collect_member_names(b, &qname))
            .unwrap_or_default();

        if let Some(ref ty) = typed_by {
            self.relationships.push(SysmlRelationship {
                source: qname.clone(),
                target: ty.clone(),
                kind: RelationshipKind::TypedBy.to_string(),
                file_path: self.file_path.clone(),
                span: span.clone(),
            });
        }

        let parent_qname = if self.name_stack.len() > 1 {
            Some(self.name_stack[..self.name_stack.len() - 1].join("::"))
        } else {
            None
        };

        if let Some(ref pq) = parent_qname {
            self.relationships.push(SysmlRelationship {
                source: pq.clone(),
                target: qname.clone(),
                kind: RelationshipKind::Member.to_string(),
                file_path: self.file_path.clone(),
                span: span.clone(),
            });
        }

        self.extract_definition_relationships(&node, &qname);
        self.extract_binding_value(&node, &qname);

        let elem = SysmlElement {
            qualified_name: qname.clone(),
            kind: kind.to_string(),
            file_path: self.file_path.clone(),
            span,
            doc,
            attributes: Vec::new(),
            members,
            layer: crate::vocabulary::classify_layer(kind),
        };
        self.elements.push(elem);

        if let Some(body) = find_body_child(&node) {
            self.extract_body_relationships(&body, &qname);
            for i in 0..body.child_count() {
                if let Some(child) = body.child(i) {
                    if child.is_named() {
                        self.walk_node(child);
                    }
                }
            }
        }

        self.name_stack.pop();
    }

    fn collect_member_names(&self, body: &Node, parent_qname: &str) -> Vec<String> {
        let mut names = Vec::new();
        for i in 0..body.child_count() {
            if let Some(child) = body.child(i) {
                if is_element_node(child.kind()) {
                    if let Some(name) = extract_element_name(&child, self.source) {
                        names.push(format!("{}::{}", parent_qname, name));
                    }
                }
            }
        }
        names
    }

    fn extract_definition_relationships(&mut self, node: &Node, qname: &str) {
        if let Some(spec) = find_child_by_kind(node, "definition_specialization") {
            let span = node_span(&spec);
            for qn in find_children_by_kind(&spec, "qualified_name") {
                if let Some(target) = extract_qualified_name_text(&qn, self.source) {
                    self.relationships.push(SysmlRelationship {
                        source: qname.to_string(),
                        target,
                        kind: RelationshipKind::Specialize.to_string(),
                        file_path: self.file_path.clone(),
                        span: span.clone(),
                    });
                }
            }
        }

        if let Some(decl) = find_child_by_kind(node, "usage_declaration") {
            if let Some(rel_part) = find_child_by_kind(&decl, "relationship_part") {
                if let Some(redef) = find_child_by_kind(&rel_part, "redefinition_part") {
                    let span = node_span(&redef);
                    if let Some(fc) = find_child_by_kind(&redef, "feature_chain") {
                        if let Some(target) = extract_feature_chain_text(&fc, self.source) {
                            self.relationships.push(SysmlRelationship {
                                source: qname.to_string(),
                                target,
                                kind: RelationshipKind::Redefine.to_string(),
                                file_path: self.file_path.clone(),
                                span,
                            });
                        }
                    }
                }
                if let Some(spec) = find_child_by_kind(&rel_part, "specialization_part") {
                    let span = node_span(&spec);
                    for fc in find_children_by_kind(&spec, "feature_chain") {
                        if let Some(target) = extract_feature_chain_text(&fc, self.source) {
                            self.relationships.push(SysmlRelationship {
                                source: qname.to_string(),
                                target,
                                kind: RelationshipKind::Specialize.to_string(),
                                file_path: self.file_path.clone(),
                                span: span.clone(),
                            });
                        }
                    }
                    for qn in find_children_by_kind(&spec, "qualified_name") {
                        if let Some(target) = extract_qualified_name_text(&qn, self.source) {
                            self.relationships.push(SysmlRelationship {
                                source: qname.to_string(),
                                target,
                                kind: RelationshipKind::Specialize.to_string(),
                                file_path: self.file_path.clone(),
                                span: span.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    fn extract_binding_value(&mut self, node: &Node, qname: &str) {
        let value = find_child_by_kind(node, "value_part").or_else(|| {
            find_child_by_kind(node, "usage_declaration").and_then(|decl| {
                find_child_by_kind(&decl, "relationship_part").and_then(|rp| {
                    find_child_by_kind(&rp, "redefinition_part")
                        .and_then(|rd| find_child_by_kind(&rd, "value_part"))
                })
            })
        });
        if let Some(vp) = value {
            let target = extract_reference_text(&vp, self.source)
                .or_else(|| extract_binding_value_text(&vp, self.source));
            if let Some(target) = target {
                self.relationships.push(SysmlRelationship {
                    source: qname.to_string(),
                    target,
                    kind: RelationshipKind::Bind.to_string(),
                    file_path: self.file_path.clone(),
                    span: node_span(&vp),
                });
            }
        }
    }

    fn extract_body_relationships(&mut self, body: &Node, context_qname: &str) {
        for i in 0..body.child_count() {
            if let Some(child) = body.child(i) {
                if !child.is_named() {
                    continue;
                }
                let kind = child.kind();
                if let Some(rel_kind) = dispatch_relationship_kind(kind) {
                    self.extract_relationship(&child, rel_kind, context_qname);
                }
                if kind == "allocate_statement" {
                    self.extract_nested_allocates(&child, context_qname);
                }
            }
        }
    }

    fn extract_relationship(
        &mut self,
        node: &Node,
        rel_kind: RelationshipKind,
        context_qname: &str,
    ) {
        let span = node_span(node);
        let file_path = self.file_path.clone();

        match rel_kind {
            RelationshipKind::Satisfy => {
                let qnames = find_children_by_kind(node, "qualified_name");
                let fchains = find_children_by_kind(node, "feature_chain");
                let target = qnames
                    .first()
                    .and_then(|n| extract_qualified_name_text(n, self.source))
                    .unwrap_or_default();
                let source = fchains
                    .first()
                    .and_then(|n| extract_feature_chain_text(n, self.source))
                    .unwrap_or_else(|| context_qname.to_string());
                if !target.is_empty() {
                    self.relationships.push(SysmlRelationship {
                        source,
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            RelationshipKind::Verify => {
                if let Some(target) = extract_reference_text(node, self.source) {
                    self.relationships.push(SysmlRelationship {
                        source: context_qname.to_string(),
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            RelationshipKind::Import => {
                if let Some(import_ref) = find_child_by_kind(node, "import_reference") {
                    let target = if let Some(wildcard) =
                        find_child_by_kind(&import_ref, "wildcard_import")
                    {
                        find_child_by_kind(&wildcard, "name").and_then(|n| {
                            n.utf8_text(self.source.as_bytes())
                                .ok()
                                .map(|s| format!("{}::*", strip_quotes(s)))
                        })
                    } else if let Some(qn) = find_child_by_kind(&import_ref, "qualified_name") {
                        extract_qualified_name_text(&qn, self.source)
                    } else {
                        None
                    };
                    if let Some(target) = target {
                        self.relationships.push(SysmlRelationship {
                            source: context_qname.to_string(),
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                }
            }
            RelationshipKind::Connect => {
                let endpoints = find_children_by_kind(node, "connect_endpoint");
                let texts: Vec<String> = endpoints
                    .iter()
                    .filter_map(|ep| {
                        find_child_by_kind(ep, "feature_chain")
                            .and_then(|fc| extract_feature_chain_text(&fc, self.source))
                    })
                    .collect();
                if texts.len() >= 2 {
                    self.relationships.push(SysmlRelationship {
                        source: texts[0].clone(),
                        target: texts[1].clone(),
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            RelationshipKind::Allocate => {
                let fchains = find_children_by_kind(node, "feature_chain");
                if fchains.len() >= 2 {
                    let source =
                        extract_feature_chain_text(&fchains[0], self.source).unwrap_or_default();
                    let target =
                        extract_feature_chain_text(&fchains[1], self.source).unwrap_or_default();
                    if !source.is_empty() && !target.is_empty() {
                        self.relationships.push(SysmlRelationship {
                            source,
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                }
            }
            RelationshipKind::Flow => {
                let fchains = if node.kind() == "flow_usage" {
                    find_child_by_kind(node, "flow_part")
                        .map(|fp| find_children_by_kind(&fp, "feature_chain"))
                        .unwrap_or_default()
                } else {
                    find_children_by_kind(node, "feature_chain")
                };
                if fchains.len() >= 2 {
                    let source =
                        extract_feature_chain_text(&fchains[0], self.source).unwrap_or_default();
                    let target =
                        extract_feature_chain_text(&fchains[1], self.source).unwrap_or_default();
                    if !source.is_empty() && !target.is_empty() {
                        self.relationships.push(SysmlRelationship {
                            source,
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                }
            }
            RelationshipKind::Dependency => {
                let qnames = find_children_by_kind(node, "qualified_name");
                if qnames.len() >= 2 {
                    let source =
                        extract_qualified_name_text(&qnames[0], self.source).unwrap_or_default();
                    for qn in &qnames[1..] {
                        if let Some(target) = extract_qualified_name_text(qn, self.source) {
                            self.relationships.push(SysmlRelationship {
                                source: source.clone(),
                                target,
                                kind: rel_kind.to_string(),
                                file_path: file_path.clone(),
                                span: span.clone(),
                            });
                        }
                    }
                }
            }
            RelationshipKind::Perform => {
                if let Some(target) = find_child_by_kind(node, "feature_chain")
                    .and_then(|fc| extract_feature_chain_text(&fc, self.source))
                {
                    self.relationships.push(SysmlRelationship {
                        source: context_qname.to_string(),
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            RelationshipKind::Exhibit => {
                let target = find_child_by_kind(node, "feature_chain")
                    .and_then(|fc| extract_feature_chain_text(&fc, self.source))
                    .or_else(|| {
                        find_child_by_kind(node, "qualified_name")
                            .and_then(|qn| extract_qualified_name_text(&qn, self.source))
                    });
                if let Some(target) = target {
                    self.relationships.push(SysmlRelationship {
                        source: context_qname.to_string(),
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            RelationshipKind::Succession => {
                let node_kind = node.kind();
                if node_kind == "succession_statement" {
                    let fchains = find_children_by_kind(node, "feature_chain");
                    if fchains.len() >= 2 {
                        let source = extract_feature_chain_text(&fchains[0], self.source)
                            .unwrap_or_default();
                        let target = extract_feature_chain_text(&fchains[1], self.source)
                            .unwrap_or_default();
                        if !source.is_empty() && !target.is_empty() {
                            self.relationships.push(SysmlRelationship {
                                source,
                                target,
                                kind: rel_kind.to_string(),
                                file_path,
                                span,
                            });
                        }
                    }
                } else if node_kind == "first_statement" {
                    let fchains = find_children_by_kind(node, "feature_chain");
                    if fchains.len() >= 2 {
                        let source = extract_feature_chain_text(&fchains[0], self.source)
                            .unwrap_or_default();
                        let target = extract_feature_chain_text(&fchains[1], self.source)
                            .unwrap_or_default();
                        if !source.is_empty() && !target.is_empty() {
                            self.relationships.push(SysmlRelationship {
                                source,
                                target,
                                kind: rel_kind.to_string(),
                                file_path,
                                span,
                            });
                        }
                    } else if let Some(target) = find_child_by_kind(node, "name").and_then(|n| {
                        n.utf8_text(self.source.as_bytes())
                            .ok()
                            .map(|s| strip_quotes(s).to_string())
                    }) {
                        self.relationships.push(SysmlRelationship {
                            source: context_qname.to_string(),
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                } else if let Some(target) = find_child_by_kind(node, "name").and_then(|n| {
                    n.utf8_text(self.source.as_bytes())
                        .ok()
                        .map(|s| strip_quotes(s).to_string())
                }) {
                    self.relationships.push(SysmlRelationship {
                        source: context_qname.to_string(),
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            RelationshipKind::Message => {
                let fchains = find_children_by_kind(node, "feature_chain");
                if fchains.len() >= 2 {
                    let source =
                        extract_feature_chain_text(&fchains[0], self.source).unwrap_or_default();
                    let target =
                        extract_feature_chain_text(&fchains[1], self.source).unwrap_or_default();
                    if !source.is_empty() && !target.is_empty() {
                        self.relationships.push(SysmlRelationship {
                            source,
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                } else if let Some(target) = extract_reference_text(node, self.source) {
                    self.relationships.push(SysmlRelationship {
                        source: context_qname.to_string(),
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            RelationshipKind::Stream => {
                let fchains = find_children_by_kind(node, "feature_chain");
                if fchains.len() >= 2 {
                    let source =
                        extract_feature_chain_text(&fchains[0], self.source).unwrap_or_default();
                    let target =
                        extract_feature_chain_text(&fchains[1], self.source).unwrap_or_default();
                    if !source.is_empty() && !target.is_empty() {
                        self.relationships.push(SysmlRelationship {
                            source,
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                }
            }
            RelationshipKind::Accept => {
                if let Some(trigger) = find_child_by_kind(node, "trigger_kind") {
                    if let Some(target) = extract_reference_text(&trigger, self.source) {
                        self.relationships.push(SysmlRelationship {
                            source: context_qname.to_string(),
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                }
            }
            RelationshipKind::Assert => {
                let mut found_nested = false;
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        let ck = child.kind();
                        if ck == "satisfy_statement" {
                            self.extract_relationship(
                                &child,
                                RelationshipKind::Satisfy,
                                context_qname,
                            );
                            found_nested = true;
                        } else if ck == "verify_statement" {
                            self.extract_relationship(
                                &child,
                                RelationshipKind::Verify,
                                context_qname,
                            );
                            found_nested = true;
                        }
                    }
                }
                if !found_nested {
                    if let Some(target) = extract_reference_text(node, self.source) {
                        self.relationships.push(SysmlRelationship {
                            source: context_qname.to_string(),
                            target,
                            kind: rel_kind.to_string(),
                            file_path,
                            span,
                        });
                    }
                }
            }
            RelationshipKind::Redefine if node.kind() == "redefines_statement" => {
                let qnames = find_children_by_kind(node, "qualified_name");
                let fchains = find_children_by_kind(node, "feature_chain");
                for qn in &qnames {
                    if let Some(target) = extract_qualified_name_text(qn, self.source) {
                        self.relationships.push(SysmlRelationship {
                            source: context_qname.to_string(),
                            target,
                            kind: rel_kind.to_string(),
                            file_path: file_path.clone(),
                            span: span.clone(),
                        });
                    }
                }
                for fc in &fchains {
                    if let Some(target) = extract_feature_chain_text(fc, self.source) {
                        self.relationships.push(SysmlRelationship {
                            source: context_qname.to_string(),
                            target,
                            kind: rel_kind.to_string(),
                            file_path: file_path.clone(),
                            span: span.clone(),
                        });
                    }
                }
            }
            RelationshipKind::Specialize if node.kind() == "specialization_statement" => {
                if let Some(target) = find_child_by_kind(node, "qualified_name")
                    .and_then(|qn| extract_qualified_name_text(&qn, self.source))
                {
                    self.relationships.push(SysmlRelationship {
                        source: context_qname.to_string(),
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
            _ => {
                if let Some(target) = extract_reference_text(node, self.source) {
                    self.relationships.push(SysmlRelationship {
                        source: context_qname.to_string(),
                        target,
                        kind: rel_kind.to_string(),
                        file_path,
                        span,
                    });
                }
            }
        }
    }

    fn extract_nested_allocates(&mut self, node: &Node, context_qname: &str) {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "allocate_statement" {
                    self.extract_relationship(&child, RelationshipKind::Allocate, context_qname);
                    self.extract_nested_allocates(&child, context_qname);
                }
            }
        }
    }
}

pub fn collect_parse_errors(node: Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.is_error() {
        let start = node.start_position();
        let end = node.end_position();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("<invalid>");
        let context = if text.len() > 30 {
            format!("{}...", &text[..30])
        } else {
            text.to_string()
        };
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: format!("Syntax error near '{}'", context),
            span: Span {
                start_line: start.row as u32,
                start_col: start.column as u32,
                end_line: end.row as u32,
                end_col: end.column as u32,
            },
        });
    } else if node.is_missing() {
        let start = node.start_position();
        let end = node.end_position();
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: format!("Missing {}", node.kind()),
            span: Span {
                start_line: start.row as u32,
                start_col: start.column as u32,
                end_line: end.row as u32,
                end_col: end.column as u32,
            },
        });
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_parse_errors(child, source, diagnostics);
        }
    }
}
