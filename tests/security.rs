use sci_family_pilot::assistant::tools::default_tool_catalog;

#[test]
fn every_mutating_ai_tool_requires_confirmation() {
    for tool in default_tool_catalog() {
        if !tool.read_only { assert!(tool.confirmation_required, "tool mutatif sans confirmation: {}", tool.code); }
    }
}

#[test]
fn no_ai_tool_is_authorized_as_owner_by_default() {
    for tool in default_tool_catalog() {
        assert!(!tool.authorized_roles.iter().any(|r| r == "OWNER"), "OWNER implicite dans {}", tool.code);
    }
}
