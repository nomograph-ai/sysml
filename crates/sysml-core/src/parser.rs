use std::path::Path;

use nomograph_core::traits::Parser;
use nomograph_core::types::{Diagnostic, ParseResult};

use crate::element::SysmlElement;
use crate::relationship::SysmlRelationship;
use crate::walker::{collect_parse_errors, Walker};

pub struct SysmlParser;

impl SysmlParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SysmlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for SysmlParser {
    type Elem = SysmlElement;
    type Rel = SysmlRelationship;
    type Error = nomograph_core::CoreError;

    fn parse(
        &self,
        source: &str,
        path: &Path,
    ) -> Result<ParseResult<Self::Elem, Self::Rel>, Self::Error> {
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser
            .set_language(&tree_sitter_sysml::LANGUAGE.into())
            .map_err(|e| nomograph_core::CoreError::Parse(e.to_string()))?;

        let tree = ts_parser
            .parse(source, None)
            .ok_or_else(|| nomograph_core::CoreError::Parse("Parser returned None".to_string()))?;

        let root = tree.root_node();

        let mut diagnostics = Vec::new();
        collect_parse_errors(root, source, &mut diagnostics);

        let mut walker = Walker::new(source, path.to_path_buf());
        walker.walk_root(root);

        Ok(ParseResult {
            elements: walker.elements,
            relationships: walker.relationships,
            diagnostics,
        })
    }

    fn validate(&self, source: &str) -> Vec<Diagnostic> {
        let mut ts_parser = tree_sitter::Parser::new();
        if ts_parser
            .set_language(&tree_sitter_sysml::LANGUAGE.into())
            .is_err()
        {
            return vec![Diagnostic {
                severity: nomograph_core::types::Severity::Error,
                message: "Failed to initialize parser".to_string(),
                span: nomograph_core::types::Span {
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                },
            }];
        }

        let Some(tree) = ts_parser.parse(source, None) else {
            return vec![Diagnostic {
                severity: nomograph_core::types::Severity::Error,
                message: "Parser returned None".to_string(),
                span: nomograph_core::types::Span {
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                },
            }];
        };

        let mut diagnostics = Vec::new();
        collect_parse_errors(tree.root_node(), source, &mut diagnostics);
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nomograph_core::types::Severity;
    use std::path::PathBuf;

    fn test_path() -> PathBuf {
        PathBuf::from("test.sysml")
    }

    fn eve_fixture(name: &str) -> String {
        let base = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}/../../tests/fixtures/eve/DomainModel/{}", base, name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path, e))
    }

    #[test]
    fn test_parse_simple_package() {
        let parser = SysmlParser::new();
        let source = "package Test { part def Engine; }";
        let result = parser.parse(source, &test_path()).unwrap();

        assert!(!result.elements.is_empty());
        let names: Vec<&str> = result
            .elements
            .iter()
            .map(|e| e.qualified_name.as_str())
            .collect();
        assert!(names.contains(&"Test"), "missing Test package");
        assert!(names.contains(&"Test::Engine"), "missing Test::Engine");
    }

    #[test]
    fn test_parse_element_kinds() {
        let parser = SysmlParser::new();
        let source = "package Test { part def Engine; }";
        let result = parser.parse(source, &test_path()).unwrap();

        let engine = result
            .elements
            .iter()
            .find(|e| e.qualified_name == "Test::Engine")
            .unwrap();
        assert_eq!(engine.kind, "part_definition");
    }

    #[test]
    fn test_parse_produces_relationships() {
        let parser = SysmlParser::new();
        let source = "package Test { part def Engine; part engine : Engine; }";
        let result = parser.parse(source, &test_path()).unwrap();

        assert!(!result.relationships.is_empty());
        let has_typed_by = result.relationships.iter().any(|r| r.kind == "TypedBy");
        assert!(has_typed_by, "should have TypedBy relationship");
    }

    #[test]
    fn test_validate_valid_sysml() {
        let parser = SysmlParser::new();
        let source = "package Test { part def Engine; }";
        let diagnostics = parser.validate(source);

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "valid SysML should have no errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_invalid_sysml() {
        let parser = SysmlParser::new();
        let source = "this is not sysml {{{";
        let diagnostics = parser.validate(source);

        let has_errors = diagnostics.iter().any(|d| d.severity == Severity::Error);
        assert!(has_errors, "invalid SysML should have errors");
    }

    #[test]
    fn test_parse_eve_mining_frigate() {
        let parser = SysmlParser::new();
        let source = eve_fixture("MiningFrigate.sysml");
        let path = PathBuf::from("MiningFrigate.sysml");
        let result = parser.parse(&source, &path).unwrap();

        assert!(
            !result.elements.is_empty(),
            "should extract elements from MiningFrigate.sysml"
        );
        assert!(
            !result.relationships.is_empty(),
            "should extract relationships"
        );
    }

    #[test]
    fn test_parse_eve_requirements() {
        let parser = SysmlParser::new();
        let source = eve_fixture("MiningFrigateRequirements.sysml");
        let path = PathBuf::from("MiningFrigateRequirements.sysml");
        let result = parser.parse(&source, &path).unwrap();

        assert!(!result.elements.is_empty());
        let req_elements: Vec<_> = result
            .elements
            .iter()
            .filter(|e| e.kind.contains("requirement"))
            .collect();
        assert!(!req_elements.is_empty(), "should have requirement elements");
    }

    #[test]
    fn test_parse_file_path_stored() {
        let parser = SysmlParser::new();
        let source = "package Test { part def Engine; }";
        let path = PathBuf::from("my/test.sysml");
        let result = parser.parse(source, &path).unwrap();

        for elem in &result.elements {
            assert_eq!(elem.file_path, path);
        }
    }
}
