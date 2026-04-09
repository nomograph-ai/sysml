use crate::core_traits::Vocabulary;

const ELEMENT_KIND_MAP: &[(&str, &[&str])] = &[
    (
        "requirement",
        &["requirement_definition", "requirement_usage"],
    ),
    ("part", &["part_definition", "part_usage"]),
    ("port", &["port_definition", "port_usage"]),
    ("connection", &["connection_definition", "connection_usage"]),
    ("interface", &["connection_definition", "connection_usage"]),
    ("constraint", &["constraint_definition", "constraint_usage"]),
    ("action", &["action_definition", "action_usage"]),
    ("behavior", &["action_definition", "action_usage"]),
    ("state", &["state_definition", "state_usage"]),
    ("mode", &["state_definition", "state_usage"]),
    ("attribute", &["attribute_definition", "attribute_usage"]),
    ("property", &["attribute_definition", "attribute_usage"]),
    ("use_case", &["use_case_definition", "use_case_usage"]),
    (
        "analysis",
        &["analysis_case_definition", "analysis_case_usage"],
    ),
    ("view", &["view_definition", "view_usage"]),
    ("viewpoint", &["viewpoint_definition"]),
    ("concern", &["concern_definition", "concern_usage"]),
    ("stakeholder", &["stakeholder_usage"]),
    (
        "enumeration",
        &["enumeration_definition", "enumeration_usage"],
    ),
    ("enum", &["enumeration_definition", "enumeration_usage"]),
    ("calc", &["calc_definition", "calc_usage"]),
    ("calculation", &["calc_definition", "calc_usage"]),
    ("item", &["item_definition", "item_usage"]),
    ("metadata", &["metadata_definition", "metadata_usage"]),
    ("annotation", &["metadata_definition", "metadata_usage"]),
    ("flow", &["item_flow_usage"]),
    ("allocation", &["part_usage"]),
    ("package", &["package_definition", "library_package"]),
];

const SYSML_VOCABULARY: &[(&[&str], &[&str])] = &[
    (
        &["requirement", "req"],
        &["requirement_definition", "requirement_usage"],
    ),
    (&["satisfy", "satisfaction"], &["Satisfy"]),
    (&["verify", "verification"], &["Verify"]),
    (&["allocate", "allocation"], &["Allocate"]),
    (
        &["connect", "connection", "interface"],
        &["connection_definition", "connection_usage", "Connect"],
    ),
    (&["port"], &["port_definition", "port_usage"]),
    (&["flow"], &["Flow"]),
    (
        &["constraint"],
        &["constraint_definition", "constraint_usage"],
    ),
    (
        &["action", "behavior"],
        &["action_definition", "action_usage"],
    ),
    (&["state", "mode"], &["state_definition", "state_usage"]),
    (&["part", "component"], &["part_definition", "part_usage"]),
    (
        &["attribute", "property"],
        &["attribute_definition", "attribute_usage"],
    ),
    (&["import", "dependency"], &["Import", "Dependency"]),
    (&["performance", "perform"], &["Perform"]),
    (&["use case"], &["use_case_definition", "use_case_usage"]),
    (
        &["analysis"],
        &["analysis_case_definition", "analysis_case_usage"],
    ),
    (
        &["view", "viewpoint"],
        &["view_definition", "view_usage", "viewpoint_definition"],
    ),
    (
        &["concern", "stakeholder"],
        &["concern_definition", "concern_usage", "stakeholder_usage"],
    ),
    (
        &["enumeration", "enum"],
        &["enumeration_definition", "enumeration_usage"],
    ),
    (&["calculation", "calc"], &["calc_definition", "calc_usage"]),
    (&["item"], &["item_definition", "item_usage"]),
    (
        &["metadata", "annotation"],
        &["metadata_definition", "metadata_usage"],
    ),
];

pub(crate) const STRUCTURAL_RELATIONSHIP_KINDS: &[&str] = &["Import", "Member"];

pub(crate) const RELATIONSHIP_KIND_NAMES: &[&str] = &[
    "Satisfy",
    "Verify",
    "Import",
    "Specialize",
    "Allocate",
    "Connect",
    "Bind",
    "Flow",
    "Stream",
    "Dependency",
    "Redefine",
    "Expose",
    "Perform",
    "Exhibit",
    "Include",
    "Succession",
    "Transition",
    "Send",
    "Accept",
    "Require",
    "Assume",
    "Assert",
    "Assign",
    "Subject",
    "Render",
    "Frame",
    "Message",
    "TypedBy",
    "Member",
];

pub(crate) const ELEMENT_KIND_NAMES: &[&str] = &[
    "requirement_definition",
    "requirement_usage",
    "part_definition",
    "part_usage",
    "port_definition",
    "port_usage",
    "connection_definition",
    "connection_usage",
    "constraint_definition",
    "constraint_usage",
    "action_definition",
    "action_usage",
    "state_definition",
    "state_usage",
    "attribute_definition",
    "attribute_usage",
    "use_case_definition",
    "use_case_usage",
    "analysis_case_definition",
    "analysis_case_usage",
    "view_definition",
    "view_usage",
    "viewpoint_definition",
    "concern_definition",
    "concern_usage",
    "stakeholder_usage",
    "enumeration_definition",
    "enumeration_usage",
    "calc_definition",
    "calc_usage",
    "item_definition",
    "item_usage",
    "metadata_definition",
    "metadata_usage",
    "item_flow_usage",
    "interface_definition",
    "interface_usage",
    "verification_definition",
    "verification_usage",
    "analysis_definition",
    "analysis_usage",
    "occurrence_definition",
    "actor_usage",
    "objective_usage",
    "event_occurrence_usage",
    "exhibit_usage",
    "end_usage",
    "parameter_usage",
    "generic_usage",
    "feature_usage",
    "timeslice_usage",
    "snapshot_usage",
    "package_definition",
    "library_package",
];

const STOP_WORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "and",
    "or",
    "for",
    "to",
    "of",
    "in",
    "is",
    "it",
    "its",
    "are",
    "was",
    "were",
    "be",
    "been",
    "do",
    "does",
    "did",
    "has",
    "have",
    "had",
    "with",
    "that",
    "this",
    "what",
    "how",
    "which",
    "where",
    "when",
    "who",
    "why",
    "find",
    "show",
    "get",
    "list",
    "describe",
    "identify",
    "determine",
    "explain",
    "i",
    "me",
    "my",
    "we",
    "us",
    "you",
    "your",
    "use",
    "using",
    "from",
    "by",
    "not",
    "no",
    "any",
    "all",
    "each",
    "if",
    "then",
    "than",
    "but",
    "so",
    "as",
    "on",
    "at",
    "about",
    "into",
    "can",
    "should",
    "would",
    "could",
    "will",
];

const STEM_SUFFIXES: &[&str] = &[
    "ation", "ment", "ness", "tion", "sion", "ance", "ence", "ity", "ing", "ies", "ied", "ous",
    "ive", "ful", "ion", "ed", "ly", "er", "es", "al", "s",
];

#[derive(Debug, Clone)]
pub struct ExpandedQuery {
    pub original: String,
    pub tokens: Vec<String>,
    pub element_kinds: Vec<String>,
    pub relationship_kinds: Vec<String>,
}

fn naive_stem(word: &str) -> &str {
    for suffix in STEM_SUFFIXES {
        if let Some(stem) = word.strip_suffix(suffix) {
            if stem.len() >= 3 {
                return stem;
            }
        }
    }
    word
}

fn stem_match(token: &str, term: &str) -> bool {
    let t_stem = naive_stem(token);
    let v_stem = naive_stem(term);
    t_stem == v_stem
        || t_stem.starts_with(v_stem)
        || v_stem.starts_with(t_stem)
        || token.starts_with(term)
        || term.starts_with(token)
}

pub fn expand_query(query: &str) -> ExpandedQuery {
    let lower = query.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && !STOP_WORDS.contains(s))
        .map(|s| s.to_string())
        .collect();

    let mut element_kinds = Vec::new();
    let mut relationship_kinds = Vec::new();

    for (terms, mappings) in SYSML_VOCABULARY {
        let matched = terms.iter().any(|term| {
            if term.contains(' ') {
                lower.contains(term)
            } else {
                tokens.iter().any(|t| t == term || stem_match(t, term))
            }
        });

        if matched {
            for mapping in *mappings {
                let is_rel = RELATIONSHIP_KIND_NAMES
                    .iter()
                    .any(|rk| rk.eq_ignore_ascii_case(mapping));
                if is_rel {
                    if !relationship_kinds.contains(&mapping.to_string()) {
                        relationship_kinds.push(mapping.to_string());
                    }
                } else if !element_kinds.contains(&mapping.to_string()) {
                    element_kinds.push(mapping.to_string());
                }
            }
        }
    }

    ExpandedQuery {
        original: query.to_string(),
        tokens,
        element_kinds,
        relationship_kinds,
    }
}

use crate::element::RflpLayer;

pub fn classify_layer(kind: &str) -> Option<RflpLayer> {
    match kind {
        "requirement_definition"
        | "requirement_usage"
        | "concern_definition"
        | "concern_usage"
        | "stakeholder_usage"
        | "constraint_definition"
        | "constraint_usage"
        | "verification_definition"
        | "verification_usage"
        | "objective_usage" => Some(RflpLayer::Requirements),
        "use_case_definition"
        | "use_case_usage"
        | "action_definition"
        | "action_usage"
        | "state_definition"
        | "state_usage"
        | "calc_definition"
        | "calc_usage"
        | "analysis_case_definition"
        | "analysis_case_usage"
        | "analysis_definition"
        | "analysis_usage"
        | "item_flow_usage"
        | "actor_usage"
        | "event_occurrence_usage"
        | "timeslice_usage"
        | "snapshot_usage"
        | "exhibit_usage" => Some(RflpLayer::Functional),
        "part_definition"
        | "part_usage"
        | "port_definition"
        | "port_usage"
        | "connection_definition"
        | "connection_usage"
        | "interface_definition"
        | "interface_usage"
        | "attribute_definition"
        | "attribute_usage"
        | "item_definition"
        | "item_usage"
        | "occurrence_definition"
        | "end_usage"
        | "parameter_usage" => Some(RflpLayer::Logical),
        _ => None,
    }
}

pub struct SysmlVocabulary;

impl Vocabulary for SysmlVocabulary {
    fn expand_kind(&self, kind: &str) -> Vec<&str> {
        let lower = kind.to_lowercase();
        for (key, kinds) in ELEMENT_KIND_MAP {
            if *key == lower.as_str() {
                return kinds.to_vec();
            }
        }
        vec![]
    }

    fn normalize_kind<'a>(&self, kind: &'a str) -> &'a str {
        for (normalized, specifics) in ELEMENT_KIND_MAP {
            if specifics.contains(&kind) {
                return normalized;
            }
        }
        kind
    }

    fn relationship_kinds(&self) -> &[&str] {
        RELATIONSHIP_KIND_NAMES
    }

    fn element_kinds(&self) -> &[&str] {
        ELEMENT_KIND_NAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_requirement_satisfy_query() {
        let eq = expand_query("What requirements does the engine satisfy?");
        assert!(eq
            .element_kinds
            .iter()
            .any(|k| k == "requirement_definition"),);
        assert!(eq.element_kinds.iter().any(|k| k == "requirement_usage"),);
        assert!(eq
            .relationship_kinds
            .iter()
            .any(|k| k.eq_ignore_ascii_case("Satisfy")),);
    }

    #[test]
    fn test_expand_allocate_query() {
        let eq = expand_query("How are functions allocated to components?");
        assert!(eq
            .relationship_kinds
            .iter()
            .any(|k| k.eq_ignore_ascii_case("Allocate")),);
        assert!(eq.element_kinds.iter().any(|k| k == "part_definition"),);
    }

    #[test]
    fn test_expand_multi_word_term() {
        let eq = expand_query("Show me the use case diagram");
        assert!(eq.element_kinds.iter().any(|k| k == "use_case_definition"),);
    }

    #[test]
    fn test_expand_no_matches() {
        let eq = expand_query("hello world");
        assert!(eq.element_kinds.is_empty());
        assert!(eq.relationship_kinds.is_empty());
    }

    #[test]
    fn test_expand_preserves_tokens() {
        let eq = expand_query("What requirements exist?");
        assert!(!eq.tokens.contains(&"what".to_string()));
        assert!(eq.tokens.contains(&"requirements".to_string()));
        assert!(eq.tokens.contains(&"exist".to_string()));
    }

    #[test]
    fn test_expand_deduplicates() {
        let eq = expand_query("requirement req requirement");
        let req_def_count = eq
            .element_kinds
            .iter()
            .filter(|k| k.as_str() == "requirement_definition")
            .count();
        assert_eq!(req_def_count, 1);
    }

    #[test]
    fn test_stop_words_filtered() {
        let eq = expand_query("find the shield module for this system");
        assert!(!eq.tokens.contains(&"find".to_string()));
        assert!(!eq.tokens.contains(&"the".to_string()));
        assert!(!eq.tokens.contains(&"for".to_string()));
        assert!(!eq.tokens.contains(&"this".to_string()));
        assert!(eq.tokens.contains(&"shield".to_string()));
        assert!(eq.tokens.contains(&"module".to_string()));
        assert!(eq.tokens.contains(&"system".to_string()));
    }

    #[test]
    fn test_naive_stem() {
        assert_eq!(naive_stem("requirements"), "requirement");
        assert_eq!(naive_stem("satisfaction"), "satisfac");
        assert_eq!(naive_stem("verification"), "verific");
        assert_eq!(naive_stem("mining"), "min");
        assert_eq!(naive_stem("ore"), "ore");
    }

    #[test]
    fn test_stem_match_across_forms() {
        assert!(stem_match("requirements", "requirement"));
        assert!(stem_match("satisfies", "satisfy"));
        assert!(stem_match("connections", "connect"));
    }

    #[test]
    fn test_classify_layer_requirements() {
        assert_eq!(
            classify_layer("requirement_definition"),
            Some(RflpLayer::Requirements)
        );
        assert_eq!(
            classify_layer("requirement_usage"),
            Some(RflpLayer::Requirements)
        );
        assert_eq!(
            classify_layer("constraint_definition"),
            Some(RflpLayer::Requirements)
        );
        assert_eq!(
            classify_layer("concern_definition"),
            Some(RflpLayer::Requirements)
        );
        assert_eq!(
            classify_layer("stakeholder_usage"),
            Some(RflpLayer::Requirements)
        );
    }

    #[test]
    fn test_classify_layer_functional() {
        assert_eq!(
            classify_layer("action_definition"),
            Some(RflpLayer::Functional)
        );
        assert_eq!(classify_layer("action_usage"), Some(RflpLayer::Functional));
        assert_eq!(
            classify_layer("state_definition"),
            Some(RflpLayer::Functional)
        );
        assert_eq!(
            classify_layer("use_case_definition"),
            Some(RflpLayer::Functional)
        );
        assert_eq!(
            classify_layer("calc_definition"),
            Some(RflpLayer::Functional)
        );
        assert_eq!(
            classify_layer("item_flow_usage"),
            Some(RflpLayer::Functional)
        );
    }

    #[test]
    fn test_classify_layer_logical() {
        assert_eq!(classify_layer("part_definition"), Some(RflpLayer::Logical));
        assert_eq!(classify_layer("part_usage"), Some(RflpLayer::Logical));
        assert_eq!(classify_layer("port_definition"), Some(RflpLayer::Logical));
        assert_eq!(
            classify_layer("connection_definition"),
            Some(RflpLayer::Logical)
        );
        assert_eq!(
            classify_layer("attribute_definition"),
            Some(RflpLayer::Logical)
        );
        assert_eq!(classify_layer("item_definition"), Some(RflpLayer::Logical));
    }

    #[test]
    fn test_classify_layer_none() {
        assert_eq!(classify_layer("package_definition"), None);
        assert_eq!(classify_layer("library_package"), None);
        assert_eq!(classify_layer("metadata_definition"), None);
        assert_eq!(classify_layer("enumeration_definition"), None);
        assert_eq!(classify_layer("view_definition"), None);
        assert_eq!(classify_layer("unknown_kind"), None);
    }
}
