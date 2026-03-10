use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldKind {
    Requirement,
    Verification,
    Part,
    Package,
    UseCase,
    Action,
    State,
    Interface,
}

impl std::str::FromStr for ScaffoldKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "requirement" | "req" => Ok(Self::Requirement),
            "verification" | "verify" => Ok(Self::Verification),
            "part" => Ok(Self::Part),
            "package" | "pkg" => Ok(Self::Package),
            "use-case" | "usecase" | "use_case" => Ok(Self::UseCase),
            "action" => Ok(Self::Action),
            "state" => Ok(Self::State),
            "interface" => Ok(Self::Interface),
            _ => Err(format!(
                "Unknown scaffold kind: '{}'. Valid: requirement, verification, part, package, use-case, action, state, interface",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScaffoldResult {
    pub kind: String,
    pub name: String,
    pub sysml: String,
}

pub fn generate(kind: ScaffoldKind, name: &str) -> ScaffoldResult {
    let sysml = match kind {
        ScaffoldKind::Requirement => format!(
            r#"requirement def {name} {{
    doc /* TODO: describe requirement */

    attribute id : String;
    attribute text : String;
}}"#
        ),
        ScaffoldKind::Verification => format!(
            r#"verification def {name} {{
    doc /* TODO: describe verification */

    subject testSubject;

    objective {{
        doc /* TODO: verification objective */
    }}
}}"#
        ),
        ScaffoldKind::Part => format!(
            r#"part def {name} {{
    doc /* TODO: describe part */

    attribute mass : Real;

    port controlPort : ControlPort;
}}"#
        ),
        ScaffoldKind::Package => format!(
            r#"package {name} {{
    doc /* TODO: package description */

}}"#
        ),
        ScaffoldKind::UseCase => format!(
            r#"use case def {name} {{
    doc /* TODO: describe use case */

    subject systemUnderTest;

    objective {{
        doc /* TODO: use case objective */
    }}
}}"#
        ),
        ScaffoldKind::Action => format!(
            r#"action def {name} {{
    doc /* TODO: describe action */

    in item input;
    out item output;
}}"#
        ),
        ScaffoldKind::State => format!(
            r#"state def {name} {{
    doc /* TODO: describe state machine */

    entry action entryAction;
    exit action exitAction;
}}"#
        ),
        ScaffoldKind::Interface => format!(
            r#"interface def {name} {{
    doc /* TODO: describe interface */

    end port supplier;
    end port consumer;
}}"#
        ),
    };

    ScaffoldResult {
        kind: format!("{kind:?}"),
        name: name.to_string(),
        sysml,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_requirement() {
        let result = generate(ScaffoldKind::Requirement, "MFRQ99");
        assert!(result.sysml.contains("requirement def MFRQ99"));
        assert!(result.sysml.contains("attribute id"));
    }

    #[test]
    fn test_scaffold_part() {
        let result = generate(ScaffoldKind::Part, "NewModule");
        assert!(result.sysml.contains("part def NewModule"));
        assert!(result.sysml.contains("port controlPort"));
    }

    #[test]
    fn test_scaffold_package() {
        let result = generate(ScaffoldKind::Package, "NewPkg");
        assert!(result.sysml.contains("package NewPkg"));
    }

    #[test]
    fn test_scaffold_verification() {
        let result = generate(ScaffoldKind::Verification, "VerifyShield");
        assert!(result.sysml.contains("verification def VerifyShield"));
        assert!(result.sysml.contains("subject testSubject"));
    }

    #[test]
    fn test_scaffold_use_case() {
        let result = generate(ScaffoldKind::UseCase, "MiningOp");
        assert!(result.sysml.contains("use case def MiningOp"));
    }

    #[test]
    fn test_scaffold_action() {
        let result = generate(ScaffoldKind::Action, "DetectThreat");
        assert!(result.sysml.contains("action def DetectThreat"));
    }

    #[test]
    fn test_scaffold_state() {
        let result = generate(ScaffoldKind::State, "ShipStates");
        assert!(result.sysml.contains("state def ShipStates"));
    }

    #[test]
    fn test_scaffold_interface() {
        let result = generate(ScaffoldKind::Interface, "PowerInterface");
        assert!(result.sysml.contains("interface def PowerInterface"));
    }

    #[test]
    fn test_parse_kind() {
        assert_eq!(
            "requirement".parse::<ScaffoldKind>().unwrap(),
            ScaffoldKind::Requirement
        );
        assert_eq!(
            "req".parse::<ScaffoldKind>().unwrap(),
            ScaffoldKind::Requirement
        );
        assert_eq!(
            "use-case".parse::<ScaffoldKind>().unwrap(),
            ScaffoldKind::UseCase
        );
        assert!("invalid".parse::<ScaffoldKind>().is_err());
    }

    #[test]
    fn test_scaffold_parseable() {
        use nomograph_core::traits::Parser;
        let parser = crate::parser::SysmlParser::new();
        for kind in [
            ScaffoldKind::Requirement,
            ScaffoldKind::Part,
            ScaffoldKind::Package,
            ScaffoldKind::UseCase,
            ScaffoldKind::Action,
            ScaffoldKind::State,
            ScaffoldKind::Interface,
        ] {
            let result = generate(kind, "TestName");
            let parse_result = parser.parse(&result.sysml, std::path::Path::new("scaffold.sysml"));
            assert!(
                parse_result.is_ok(),
                "Scaffold {:?} produced unparseable SysML: {}",
                kind,
                result.sysml
            );
        }
    }
}
