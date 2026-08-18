use olecfsdk_test::{CoverageRatchet, audit_classic_office_file_roots};

#[test]
#[ignore = "classic Office coverage audit runs explicitly"]
fn classic_office_coverage_meets_ratchet() {
    let report = audit_classic_office_file_roots().expect("audit classic Office corpus");
    let ratchet = serde_json::from_str::<CoverageRatchet>(include_str!("../coverage-ratchet.json"))
        .expect("parse coverage ratchet");
    report
        .assert_meets(&ratchet)
        .unwrap_or_else(|error| panic!("{error}\nreport:\n{}", report.to_pretty_json().unwrap()));
}
