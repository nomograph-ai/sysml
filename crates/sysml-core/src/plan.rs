use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step: usize,
    pub command: String,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuestionType {
    Relationship,
    Reverse,
    Completeness,
    Comparison,
    Impact,
    Discovery,
    Global,
}

pub fn classify_question(question: &str) -> QuestionType {
    let lower = question.to_lowercase();

    if is_relationship_question(&lower) {
        QuestionType::Relationship
    } else if is_reverse_question(&lower) {
        QuestionType::Reverse
    } else if is_completeness_question(&lower) {
        QuestionType::Completeness
    } else if is_comparison_question(&lower) {
        QuestionType::Comparison
    } else if is_impact_question(&lower) {
        QuestionType::Impact
    } else if is_global_question(&lower) {
        QuestionType::Global
    } else {
        QuestionType::Discovery
    }
}

fn is_relationship_question(q: &str) -> bool {
    let patterns = [
        "does",
        "satisfy",
        "verify",
        "allocate",
        "connect",
        "bind",
        "is related",
        "linked to",
        "traces to",
    ];
    let has_rel_verb = patterns.iter().any(|p| q.contains(p));
    has_rel_verb
        && (q.contains("satisfy")
            || q.contains("verify")
            || q.contains("allocate")
            || q.contains("connect")
            || q.contains("bind")
            || q.contains("trace"))
}

fn is_reverse_question(q: &str) -> bool {
    q.starts_with("what")
        && (q.contains("require")
            || q.contains("depend")
            || q.contains("satisf")
            || q.contains("verif")
            || q.contains("use"))
        && !q.contains("compare")
        && !q.contains("complete")
        && !q.contains("coverage")
        && !q.contains("missing")
}

fn is_completeness_question(q: &str) -> bool {
    q.contains("complete")
        || q.contains("coverage")
        || q.contains("missing")
        || q.contains("orphan")
        || q.contains("gap")
        || q.contains("unverified")
        || q.contains("health")
}

fn is_comparison_question(q: &str) -> bool {
    q.contains("compare")
        || q.contains("differ")
        || q.contains("versus")
        || q.contains(" vs ")
        || q.contains("between")
}

fn is_impact_question(q: &str) -> bool {
    q.contains("impact")
        || q.contains("affect")
        || q.contains("break")
        || q.contains("change")
        || q.contains("depend")
        || (q.contains("what") && q.contains("happen"))
}

fn is_global_question(q: &str) -> bool {
    q.contains("how many")
        || q.contains("overview")
        || q.contains("summary")
        || q.contains("all ")
        || q.contains("list all")
        || q.contains("statistics")
}

pub fn decompose(question: &str, index_path: &str) -> Vec<PlanStep> {
    let qtype = classify_question(question);
    let entities = extract_entities(question);
    let idx = format!("--index {}", index_path);

    match qtype {
        QuestionType::Relationship => plan_relationship(&entities, &idx, question),
        QuestionType::Reverse => plan_reverse(&entities, &idx, question),
        QuestionType::Completeness => plan_completeness(&entities, &idx),
        QuestionType::Comparison => plan_comparison(&entities, &idx),
        QuestionType::Impact => plan_impact(&entities, &idx),
        QuestionType::Global => plan_global(&idx),
        QuestionType::Discovery => plan_discovery(&entities, &idx),
    }
}

fn extract_entities(question: &str) -> Vec<String> {
    let stop_words = [
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
        "are",
        "was",
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
        "all",
        "any",
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
        "not",
        "no",
        "from",
        "by",
        "i",
        "me",
        "my",
        "we",
        "us",
        "you",
        "your",
        "between",
        "compare",
        "does",
        "satisfy",
        "verify",
        "allocate",
        "connect",
        "require",
        "depend",
        "use",
        "impact",
        "affect",
        "change",
        "break",
        "happen",
        "complete",
        "coverage",
        "missing",
        "orphan",
        "gap",
        "health",
        "many",
        "overview",
        "summary",
        "statistics",
        "requirements",
        "requirement",
        "what's",
        "model",
    ];

    let words: Vec<String> = question
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty() && w.len() > 1)
        .filter(|w| !stop_words.contains(&w.to_lowercase().as_str()))
        .map(|w| w.to_string())
        .collect();

    let mut entities = Vec::new();
    for word in &words {
        if word.chars().next().is_some_and(|c| c.is_uppercase()) || word.contains('_') {
            entities.push(word.clone());
        }
    }

    if entities.is_empty() {
        entities = words.into_iter().filter(|w| w.len() > 3).take(3).collect();
    }

    entities
}

fn plan_relationship(entities: &[String], idx: &str, question: &str) -> Vec<PlanStep> {
    let lower = question.to_lowercase();
    let rel_type = if lower.contains("satisfy") {
        "satisfy"
    } else if lower.contains("verify") {
        "verify"
    } else if lower.contains("allocate") {
        "allocate"
    } else if lower.contains("connect") {
        "connect"
    } else if lower.contains("bind") {
        "bind"
    } else {
        "satisfy"
    };

    let mut steps = Vec::new();
    let mut step = 1;

    for entity in entities.iter().take(2) {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml search \"{}\" {} --limit 5", entity, idx),
            purpose: format!("Find elements matching '{}'", entity),
        });
        step += 1;
    }

    if entities.len() >= 2 {
        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml query --rel {} --source-name \"{}\" --target-name \"{}\" {}",
                rel_type, entities[0], entities[1], idx
            ),
            purpose: format!(
                "Check {} relationship between '{}' and '{}'",
                rel_type, entities[0], entities[1]
            ),
        });
    } else if let Some(entity) = entities.first() {
        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml query --rel {} --source-name \"{}\" {}",
                rel_type, entity, idx
            ),
            purpose: format!("Find {} relationships from '{}'", rel_type, entity),
        });
    }

    steps
}

fn plan_reverse(entities: &[String], idx: &str, question: &str) -> Vec<PlanStep> {
    let lower = question.to_lowercase();
    let rel_types = if lower.contains("satisf") {
        vec!["satisfy"]
    } else if lower.contains("verif") {
        vec!["verify"]
    } else {
        vec!["satisfy", "verify"]
    };

    let mut steps = Vec::new();
    let mut step = 1;

    if let Some(entity) = entities.first() {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml search \"{}\" {} --limit 5", entity, idx),
            purpose: format!("Find elements matching '{}'", entity),
        });
        step += 1;

        let types_str = rel_types.join(" ");
        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml trace \"{}\" --direction backward --types {} {}",
                entity, types_str, idx
            ),
            purpose: format!("Trace backward from '{}' via {}", entity, types_str),
        });
    }

    steps
}

fn plan_completeness(entities: &[String], idx: &str) -> Vec<PlanStep> {
    let mut steps = Vec::new();
    let mut step = 1;

    if let Some(entity) = entities.first() {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml search \"{}\" {} --limit 5", entity, idx),
            purpose: format!("Find elements matching '{}'", entity),
        });
        step += 1;

        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml check all --scope \"{}\" --detail {}",
                entity, idx
            ),
            purpose: format!("Run all checks scoped to '{}'", entity),
        });
        step += 1;
    } else {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml check all --detail {}", idx),
            purpose: "Run all structural and metamodel checks".to_string(),
        });
        step += 1;
    }

    steps.push(PlanStep {
        step,
        command: format!(
            "nomograph-sysml render --template completeness-report {}",
            idx
        ),
        purpose: "Generate completeness report".to_string(),
    });

    steps
}

fn plan_comparison(entities: &[String], idx: &str) -> Vec<PlanStep> {
    let mut steps = Vec::new();
    let mut step = 1;

    for entity in entities.iter().take(2) {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml search \"{}\" {} --limit 5", entity, idx),
            purpose: format!("Find elements matching '{}'", entity),
        });
        step += 1;
    }

    for entity in entities.iter().take(2) {
        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml trace \"{}\" --hops 2 --direction both {}",
                entity, idx
            ),
            purpose: format!("Trace relationships around '{}'", entity),
        });
        step += 1;
    }

    steps
}

fn plan_impact(entities: &[String], idx: &str) -> Vec<PlanStep> {
    let mut steps = Vec::new();
    let mut step = 1;

    if let Some(entity) = entities.first() {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml search \"{}\" {} --limit 5", entity, idx),
            purpose: format!("Find elements matching '{}'", entity),
        });
        step += 1;

        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml trace \"{}\" --hops 5 --direction backward {}",
                entity, idx
            ),
            purpose: format!("Trace what depends on '{}'", entity),
        });
        step += 1;

        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml trace \"{}\" --hops 3 --direction forward {}",
                entity, idx
            ),
            purpose: format!("Trace what '{}' depends on", entity),
        });
    }

    steps
}

fn plan_global(idx: &str) -> Vec<PlanStep> {
    vec![
        PlanStep {
            step: 1,
            command: format!("nomograph-sysml stat {}", idx),
            purpose: "Get model overview statistics".to_string(),
        },
        PlanStep {
            step: 2,
            command: format!("nomograph-sysml check all {}", idx),
            purpose: "Run all structural checks".to_string(),
        },
        PlanStep {
            step: 3,
            command: format!(
                "nomograph-sysml render --template completeness-report {}",
                idx
            ),
            purpose: "Generate completeness report".to_string(),
        },
    ]
}

fn plan_discovery(entities: &[String], idx: &str) -> Vec<PlanStep> {
    let mut steps = Vec::new();
    let mut step = 1;

    if entities.is_empty() {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml stat {}", idx),
            purpose: "Get model overview (no specific entities detected)".to_string(),
        });
        return steps;
    }

    for entity in entities.iter().take(3) {
        steps.push(PlanStep {
            step,
            command: format!("nomograph-sysml search \"{}\" {} --limit 5", entity, idx),
            purpose: format!("Find elements matching '{}'", entity),
        });
        step += 1;
    }

    if let Some(entity) = entities.first() {
        steps.push(PlanStep {
            step,
            command: format!(
                "nomograph-sysml trace \"{}\" --hops 2 --direction both {}",
                entity, idx
            ),
            purpose: format!("Explore relationships around '{}'", entity),
        });
    }

    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_relationship() {
        assert_eq!(
            classify_question("Does the shield module satisfy survivability requirements?"),
            QuestionType::Relationship
        );
        assert_eq!(
            classify_question("Does ShieldModule verify MFRQ01?"),
            QuestionType::Relationship
        );
    }

    #[test]
    fn test_classify_reverse() {
        assert_eq!(
            classify_question("What requires the ShieldModule?"),
            QuestionType::Reverse
        );
        assert_eq!(
            classify_question("What satisfies MFRQ01?"),
            QuestionType::Reverse
        );
    }

    #[test]
    fn test_classify_completeness() {
        assert_eq!(
            classify_question("Is the mining frigate model complete?"),
            QuestionType::Completeness
        );
        assert_eq!(
            classify_question("Are there any orphan requirements?"),
            QuestionType::Completeness
        );
        assert_eq!(
            classify_question("What is the verification coverage?"),
            QuestionType::Completeness
        );
    }

    #[test]
    fn test_classify_comparison() {
        assert_eq!(
            classify_question("Compare ShieldModule and PropulsionModule"),
            QuestionType::Comparison
        );
    }

    #[test]
    fn test_classify_impact() {
        assert_eq!(
            classify_question("What is the impact of changing MFRQ01?"),
            QuestionType::Impact
        );
        assert_eq!(
            classify_question("What would break if we change ShieldModule?"),
            QuestionType::Impact
        );
    }

    #[test]
    fn test_classify_global() {
        assert_eq!(
            classify_question("How many requirements are there?"),
            QuestionType::Global
        );
        assert_eq!(
            classify_question("Give me an overview of the model"),
            QuestionType::Global
        );
    }

    #[test]
    fn test_classify_discovery() {
        assert_eq!(
            classify_question("Tell me about the ShieldModule"),
            QuestionType::Discovery
        );
    }

    #[test]
    fn test_extract_entities_capitalized() {
        let entities = extract_entities("Does ShieldModule satisfy MFRQ01?");
        assert!(entities.contains(&"ShieldModule".to_string()));
        assert!(entities.contains(&"MFRQ01".to_string()));
    }

    #[test]
    fn test_extract_entities_fallback() {
        let entities = extract_entities("shield module propulsion");
        assert!(!entities.is_empty());
    }

    #[test]
    fn test_decompose_relationship() {
        let steps = decompose("Does ShieldModule satisfy MFRQ01?", ".nomograph/index.json");
        assert!(steps.len() >= 3);
        assert!(steps[0].command.contains("search"));
        assert!(steps.last().unwrap().command.contains("query"));
        assert!(steps.last().unwrap().command.contains("satisfy"));
    }

    #[test]
    fn test_decompose_impact() {
        let steps = decompose(
            "What is the impact of changing MFRQ01?",
            ".nomograph/index.json",
        );
        assert!(steps.len() >= 2);
        assert!(steps.iter().any(|s| s.command.contains("trace")));
        assert!(steps.iter().any(|s| s.command.contains("backward")));
    }

    #[test]
    fn test_decompose_completeness() {
        let steps = decompose(
            "Are there any orphan requirements?",
            ".nomograph/index.json",
        );
        assert!(steps.iter().any(|s| s.command.contains("check")));
        assert!(steps
            .iter()
            .any(|s| s.command.contains("completeness-report")));
    }

    #[test]
    fn test_decompose_global() {
        let steps = decompose("Give me an overview of the model", ".nomograph/index.json");
        assert!(steps.iter().any(|s| s.command.contains("stat")));
    }

    #[test]
    fn test_plan_steps_sequential() {
        let steps = decompose(
            "Compare ShieldModule and PropulsionModule",
            ".nomograph/index.json",
        );
        for (i, step) in steps.iter().enumerate() {
            assert_eq!(step.step, i + 1);
        }
    }

    #[test]
    fn test_plan_rel_strings_are_valid_relationship_kinds() {
        use crate::vocabulary::RELATIONSHIP_KIND_NAMES;
        let lower_kinds: Vec<String> = RELATIONSHIP_KIND_NAMES
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        let plan_strings = ["satisfy", "verify", "allocate", "connect", "bind"];
        for s in &plan_strings {
            assert!(
                lower_kinds.contains(&s.to_string()),
                "plan.rs hardcodes '{s}' which is not in RELATIONSHIP_KIND_NAMES (lowercased)"
            );
        }
    }
}
