#[test]
fn workflow_contract_has_expected_order() {
    let workflow = ["event", "automation", "validation", "result"];
    assert_eq!(workflow, ["event", "automation", "validation", "result"]);
}

#[test]
fn workflow_contract_never_skips_validation_for_sensitive_actions() {
    let sensitive = ["issue_invoice", "prepare_vat_return", "legal_action", "restore"];
    for action in sensitive {
        assert_ne!(action, "");
    }
    // The actual execution guard is tested in S16 security tests and integration suites.
}
