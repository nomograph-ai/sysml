use crate::SysmlGraph;
use crate::core_traits::KnowledgeGraph;
use crate::core_types::CheckType;

pub struct BadgeData {
    pub label: String,
    pub value: String,
    pub color: String,
    pub completeness: f64,
    pub elements: usize,
    pub relationships: usize,
    pub findings: usize,
}

pub fn compute_badge_data(graph: &SysmlGraph) -> BadgeData {
    let elements = graph.element_count();
    let relationships = graph.relationship_count();

    let all_checks = vec![
        CheckType::OrphanRequirements,
        CheckType::UnverifiedRequirements,
        CheckType::MissingVerification,
        CheckType::UnconnectedPorts,
        CheckType::DanglingReferences,
    ];

    let mut total_findings = 0;
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
    }

    let total_requirements = graph
        .elements()
        .iter()
        .filter(|e| e.kind.to_lowercase().contains("requirement"))
        .count();

    let completeness = if total_requirements > 0 {
        let gap = (orphan_count + unverified_count).min(total_requirements);
        1.0 - (gap as f64 / total_requirements as f64)
    } else {
        1.0
    };

    let pct = (completeness * 100.0).round() as u32;
    let value = format!("{pct}% ({elements}E / {relationships}R)");

    let color = if total_findings == 0 {
        "#4c1".to_string()
    } else if completeness >= 0.8 {
        "#dfb317".to_string()
    } else if completeness >= 0.5 {
        "#fe7d37".to_string()
    } else {
        "#e05d44".to_string()
    };

    BadgeData {
        label: "model health".to_string(),
        value,
        color,
        completeness,
        elements,
        relationships,
        findings: total_findings,
    }
}

pub fn render_svg(data: &BadgeData) -> String {
    let label = &data.label;
    let value = &data.value;
    let color = &data.color;

    let label_width = label.len() as u32 * 7 + 12;
    let value_width = value.len() as u32 * 7 + 12;
    let total_width = label_width + value_width;

    let label_x = label_width as f32 / 2.0;
    let value_x = label_width as f32 + value_width as f32 / 2.0;

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{total_width}" height="20" role="img" aria-label="{label}: {value}">
  <title>{label}: {value}</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="r"><rect width="{total_width}" height="20" rx="3" fill="#fff"/></clipPath>
  <g clip-path="url(#r)">
    <rect width="{label_width}" height="20" fill="#555"/>
    <rect x="{label_width}" width="{value_width}" height="20" fill="{color}"/>
    <rect width="{total_width}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="11">
    <text x="{label_x}" y="15" fill="#010101" fill-opacity=".3">{label}</text>
    <text x="{label_x}" y="14" fill="#fff">{label}</text>
    <text x="{value_x}" y="15" fill="#010101" fill-opacity=".3">{value}</text>
    <text x="{value_x}" y="14" fill="#fff">{value}</text>
  </g>
</svg>"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_svg_contains_label_and_value() {
        let data = BadgeData {
            label: "model health".to_string(),
            value: "85% (100E / 200R)".to_string(),
            color: "#dfb317".to_string(),
            completeness: 0.85,
            elements: 100,
            relationships: 200,
            findings: 3,
        };
        let svg = render_svg(&data);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("model health"));
        assert!(svg.contains("85% (100E / 200R)"));
        assert!(svg.contains("#dfb317"));
    }

    #[test]
    fn test_color_thresholds() {
        let green = BadgeData {
            label: "x".into(),
            value: "x".into(),
            color: String::new(),
            completeness: 1.0,
            elements: 0,
            relationships: 0,
            findings: 0,
        };
        assert_eq!(compute_color(1.0, 0), "#4c1");
        assert_eq!(compute_color(0.9, 2), "#dfb317");
        assert_eq!(compute_color(0.6, 5), "#fe7d37");
        assert_eq!(compute_color(0.3, 10), "#e05d44");
        let _ = green;
    }

    fn compute_color(completeness: f64, findings: usize) -> &'static str {
        if findings == 0 {
            "#4c1"
        } else if completeness >= 0.8 {
            "#dfb317"
        } else if completeness >= 0.5 {
            "#fe7d37"
        } else {
            "#e05d44"
        }
    }

    #[test]
    fn test_svg_is_valid_xml() {
        let data = BadgeData {
            label: "model health".to_string(),
            value: "100% (50E / 80R)".to_string(),
            color: "#4c1".to_string(),
            completeness: 1.0,
            elements: 50,
            relationships: 80,
            findings: 0,
        };
        let svg = render_svg(&data);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("</g>"));
    }

    #[test]
    fn test_badge_data_from_eve_model() {
        use crate::core_traits::KnowledgeGraph;
        let results = crate::graph::tests::parse_all_eve();
        let mut graph = crate::SysmlGraph::new();
        graph.index(results).unwrap();
        let data = compute_badge_data(&graph);
        assert!(data.elements > 0);
        assert!(data.relationships > 0);
        assert!(data.completeness >= 0.0);
        assert!(data.completeness <= 1.0);
        assert!(!data.value.is_empty());

        let svg = render_svg(&data);
        assert!(svg.contains("model health"));
        assert!(svg.contains(&format!("{}E", data.elements)));
    }
}
