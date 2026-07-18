//! Shared coverage and regression-ratchet support for `olecfsdk` integration tests.

mod corpus;

use std::collections::{BTreeMap, BTreeSet};

pub use corpus::audit_classic_office_file_roots;
use serde::{Deserialize, Serialize};

pub const COVERAGE_SCHEMA_VERSION: u32 = 2;
pub const COVERAGE_EVIDENCE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageDomain {
    Cfb,
    Doc,
    Xls,
    Ppt,
    Vba,
    OlePropertySet,
    OfficeArt,
    Forms,
}

pub const ALL_COVERAGE_DOMAINS: [CoverageDomain; 8] = [
    CoverageDomain::Cfb,
    CoverageDomain::Doc,
    CoverageDomain::Xls,
    CoverageDomain::Ppt,
    CoverageDomain::Vba,
    CoverageDomain::OlePropertySet,
    CoverageDomain::OfficeArt,
    CoverageDomain::Forms,
];

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageCounts {
    pub discovered: u64,
    pub excluded: u64,
    pub strict: u64,
    pub compatible: u64,
    pub rejected: u64,
    pub round_tripped: u64,
}

impl CoverageCounts {
    pub const fn supported(&self) -> u64 {
        self.strict + self.compatible
    }

    pub fn validate(&self, domain: CoverageDomain) -> Result<(), String> {
        let accounted = self.excluded + self.strict + self.compatible + self.rejected;
        if accounted != self.discovered {
            return Err(format!(
                "{domain:?} inventory is not conserved: discovered={}, accounted={accounted}",
                self.discovered
            ));
        }
        if self.round_tripped > self.supported() {
            return Err(format!(
                "{domain:?} round-trip count {} exceeds supported count {}",
                self.round_tripped,
                self.supported()
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageCategoryReport {
    #[serde(flatten)]
    pub counts: CoverageCounts,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rejection_reasons: BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub exclusion_reasons: BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub round_trip_failure_reasons: BTreeMap<String, u64>,
    /// Domain-specific stable counters such as records, bytes, or typed leaves.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metrics: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DebtDisposition {
    SpecificationOpaque,
    UnknownExtension,
    Compatibility,
    Malformed,
    TemporaryUntyped,
}

impl DebtDisposition {
    const fn name(self) -> &'static str {
        match self {
            Self::SpecificationOpaque => "specification_opaque",
            Self::UnknownExtension => "unknown_extension",
            Self::Compatibility => "compatibility",
            Self::Malformed => "malformed",
            Self::TemporaryUntyped => "temporary_untyped",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageDebtEvidence {
    pub disposition: DebtDisposition,
    pub summary: String,
    pub specification: Vec<String>,
    pub upstream: Vec<String>,
    pub corpus_examples: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageEvidenceCatalog {
    pub schema_version: u32,
    pub categories: BTreeMap<CoverageDomain, BTreeMap<String, CoverageDebtEvidence>>,
}

impl CoverageEvidenceCatalog {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COVERAGE_EVIDENCE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported coverage evidence schema version {}",
                self.schema_version
            ));
        }
        validate_domain_inventory("coverage evidence", self.categories.keys().copied())?;
        for (domain, entries) in &self.categories {
            for (key, evidence) in entries {
                if !key.starts_with(&format!("debt.{}.", evidence.disposition.name()))
                    || [".units", ".records", ".bytes"]
                        .iter()
                        .any(|suffix| key.ends_with(suffix))
                    || key.split('.').any(str::is_empty)
                {
                    return Err(format!("{domain:?} has invalid debt evidence key {key}"));
                }
                if evidence.summary.trim().is_empty()
                    || evidence.specification.is_empty()
                    || evidence.upstream.is_empty()
                    || evidence.corpus_examples.is_empty()
                    || evidence
                        .specification
                        .iter()
                        .chain(&evidence.upstream)
                        .chain(&evidence.corpus_examples)
                        .any(|value| value.trim().is_empty())
                {
                    return Err(format!("{domain:?}.{key} has incomplete debt evidence"));
                }
            }
        }
        Ok(())
    }
}

pub fn bundled_coverage_evidence() -> Result<CoverageEvidenceCatalog, String> {
    let catalog =
        serde_json::from_str::<CoverageEvidenceCatalog>(include_str!("../coverage-evidence.json"))
            .map_err(|error| format!("invalid bundled coverage evidence: {error}"))?;
    catalog.validate()?;
    Ok(catalog)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageReport {
    pub schema_version: u32,
    pub categories: BTreeMap<CoverageDomain, CoverageCategoryReport>,
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self {
            schema_version: COVERAGE_SCHEMA_VERSION,
            categories: ALL_COVERAGE_DOMAINS
                .into_iter()
                .map(|domain| (domain, CoverageCategoryReport::default()))
                .collect(),
        }
    }
}

impl CoverageReport {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COVERAGE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported coverage report schema version {}",
                self.schema_version
            ));
        }
        validate_domain_inventory("coverage report", self.categories.keys().copied())?;
        for (domain, category) in &self.categories {
            category.counts.validate(*domain)?;
            validate_disposition_metrics(*domain, category)?;
            let excluded = category.exclusion_reasons.values().sum::<u64>();
            if excluded != category.counts.excluded {
                return Err(format!(
                    "{domain:?} exclusion reasons account for {excluded}, expected {}",
                    category.counts.excluded
                ));
            }
            let rejected = category.rejection_reasons.values().sum::<u64>();
            if rejected != category.counts.rejected {
                return Err(format!(
                    "{domain:?} rejection reasons account for {rejected}, expected {}",
                    category.counts.rejected
                ));
            }
            let round_trip_failures = category.round_trip_failure_reasons.values().sum::<u64>();
            if category.counts.round_tripped + round_trip_failures != category.counts.supported() {
                return Err(format!(
                    "{domain:?} round-trip outcomes do not account for every supported file"
                ));
            }
        }
        Ok(())
    }

    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self).map(|mut json| {
            json.push('\n');
            json
        })
    }

    pub fn assert_meets(&self, ratchet: &CoverageRatchet) -> Result<(), String> {
        self.validate()?;
        if ratchet.schema_version != self.schema_version {
            return Err(format!(
                "coverage schema differs: report={}, ratchet={}",
                self.schema_version, ratchet.schema_version
            ));
        }
        validate_domain_inventory("coverage ratchet", ratchet.categories.keys().copied())?;
        let mut regressions = Vec::new();
        for (domain, threshold) in &ratchet.categories {
            let Some(actual) = self.categories.get(domain) else {
                regressions.push(format!("{domain:?}: category is missing"));
                continue;
            };
            threshold.collect_regressions(*domain, actual, &mut regressions);
        }
        if regressions.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "coverage ratchet regressed:\n{}",
                regressions.join("\n")
            ))
        }
    }

    pub fn assert_debt_has_evidence(
        &self,
        catalog: &CoverageEvidenceCatalog,
    ) -> Result<(), String> {
        catalog.validate()?;
        let mut missing = BTreeSet::new();
        for (domain, category) in &self.categories {
            let evidence = catalog
                .categories
                .get(domain)
                .expect("validated evidence catalog has every domain");
            for metric in category
                .metrics
                .keys()
                .filter(|metric| metric.starts_with("debt."))
            {
                let Some(family) = debt_metric_family(metric) else {
                    missing.insert(format!("{domain:?}.{metric}: invalid debt metric measure"));
                    continue;
                };
                if !evidence.contains_key(family) {
                    missing.insert(format!("{domain:?}.{family}: debt has no evidence"));
                }
            }
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing.into_iter().collect::<Vec<_>>().join("\n"))
        }
    }
}

fn debt_metric_family(metric: &str) -> Option<&str> {
    [".units", ".records", ".bytes"]
        .into_iter()
        .find_map(|suffix| metric.strip_suffix(suffix))
}

fn validate_disposition_metrics(
    domain: CoverageDomain,
    category: &CoverageCategoryReport,
) -> Result<(), String> {
    const DISPOSITIONS: [&str; 7] = [
        "typed",
        "specification_opaque",
        "external_leaf",
        "unknown_extension",
        "compatibility",
        "malformed",
        "temporary_untyped",
    ];
    const MEASURES: [&str; 3] = ["units", "records", "bytes"];
    for metric in category
        .metrics
        .keys()
        .filter(|metric| metric.starts_with("disposition."))
    {
        let parts = metric.split('.').collect::<Vec<_>>();
        if parts.len() != 3 || !DISPOSITIONS.contains(&parts[1]) || !MEASURES.contains(&parts[2]) {
            return Err(format!(
                "{domain:?} has invalid disposition metric {metric}"
            ));
        }
    }

    let classified_records = category
        .metrics
        .iter()
        .filter(|(metric, _)| metric.starts_with("disposition.") && metric.ends_with(".records"))
        .map(|(_, count)| *count)
        .sum::<u64>();
    let expected_records = match domain {
        CoverageDomain::Xls => category.metrics.get("biff_records").copied(),
        CoverageDomain::Ppt => category.metrics.get("records").copied(),
        CoverageDomain::Vba => category.metrics.get("structural_records").copied(),
        CoverageDomain::OfficeArt => Some(
            category.metrics.get("records").copied().unwrap_or_default()
                + category
                    .metrics
                    .get("complete_records_in_partial_units")
                    .copied()
                    .unwrap_or_default()
                + category
                    .metrics
                    .get("incomplete_records")
                    .copied()
                    .unwrap_or_default(),
        ),
        _ => None,
    };
    if let Some(expected_records) = expected_records
        && classified_records != expected_records
    {
        return Err(format!(
            "{domain:?} disposition records account for {classified_records}, expected {expected_records}"
        ));
    }
    if domain == CoverageDomain::OlePropertySet {
        validate_disposition_conservation(category, domain, "units", "properties")?;
        validate_disposition_conservation(category, domain, "bytes", "property_bytes")?;
    }
    if domain == CoverageDomain::Doc {
        validate_disposition_conservation(category, domain, "units", "content_nodes")?;
    }
    if domain == CoverageDomain::Vba {
        validate_disposition_conservation(category, domain, "units", "leaf_units")?;
        validate_disposition_conservation(category, domain, "bytes", "leaf_bytes")?;
    }
    if domain == CoverageDomain::Forms {
        validate_disposition_conservation(category, domain, "units", "sites")?;
    }
    Ok(())
}

fn validate_disposition_conservation(
    category: &CoverageCategoryReport,
    domain: CoverageDomain,
    measure: &str,
    expected_metric: &str,
) -> Result<(), String> {
    let suffix = format!(".{measure}");
    let classified = category
        .metrics
        .iter()
        .filter(|(metric, _)| metric.starts_with("disposition.") && metric.ends_with(&suffix))
        .map(|(_, count)| *count)
        .sum::<u64>();
    let expected = category
        .metrics
        .get(expected_metric)
        .copied()
        .unwrap_or_default();
    if classified == expected {
        Ok(())
    } else {
        Err(format!(
            "{domain:?} disposition {measure} account for {classified}, expected {expected}"
        ))
    }
}

fn validate_domain_inventory(
    label: &str,
    actual: impl IntoIterator<Item = CoverageDomain>,
) -> Result<(), String> {
    let actual = actual.into_iter().collect::<Vec<_>>();
    let missing = ALL_COVERAGE_DOMAINS
        .into_iter()
        .filter(|domain| !actual.contains(domain))
        .collect::<Vec<_>>();
    let unexpected = actual
        .into_iter()
        .filter(|domain| !ALL_COVERAGE_DOMAINS.contains(domain))
        .collect::<Vec<_>>();
    if missing.is_empty() && unexpected.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{label} domain inventory differs: missing={missing:?}, unexpected={unexpected:?}"
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageRatchet {
    pub schema_version: u32,
    pub categories: BTreeMap<CoverageDomain, CoverageThreshold>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageThreshold {
    pub minimum_discovered: u64,
    pub minimum_strict: u64,
    pub minimum_supported: u64,
    pub minimum_round_tripped: u64,
    pub maximum_rejected: u64,
    pub maximum_excluded: u64,
    #[serde(default)]
    pub minimum_metrics: BTreeMap<String, u64>,
    #[serde(default)]
    pub maximum_metrics: BTreeMap<String, u64>,
}

impl CoverageThreshold {
    fn collect_regressions(
        &self,
        domain: CoverageDomain,
        actual: &CoverageCategoryReport,
        regressions: &mut Vec<String>,
    ) {
        let counts = &actual.counts;
        check_minimum(
            domain,
            "discovered",
            counts.discovered,
            self.minimum_discovered,
            regressions,
        );
        check_minimum(
            domain,
            "strict",
            counts.strict,
            self.minimum_strict,
            regressions,
        );
        check_minimum(
            domain,
            "supported",
            counts.supported(),
            self.minimum_supported,
            regressions,
        );
        check_minimum(
            domain,
            "round_tripped",
            counts.round_tripped,
            self.minimum_round_tripped,
            regressions,
        );
        check_maximum(
            domain,
            "rejected",
            counts.rejected,
            self.maximum_rejected,
            regressions,
        );
        for (metric, expected) in &self.minimum_metrics {
            check_minimum(
                domain,
                metric,
                actual.metrics.get(metric).copied().unwrap_or_default(),
                *expected,
                regressions,
            );
        }
        for (metric, expected) in &self.maximum_metrics {
            check_maximum(
                domain,
                metric,
                actual.metrics.get(metric).copied().unwrap_or_default(),
                *expected,
                regressions,
            );
        }
        for metric in actual
            .metrics
            .keys()
            .filter(|metric| metric.starts_with("debt."))
        {
            if !self.maximum_metrics.contains_key(metric) {
                regressions.push(format!(
                    "{domain:?}.{metric}: debt metric has no ratchet ceiling"
                ));
            }
        }
        check_maximum(
            domain,
            "excluded",
            counts.excluded,
            self.maximum_excluded,
            regressions,
        );
    }
}

fn check_minimum(
    domain: CoverageDomain,
    metric: &str,
    actual: u64,
    expected: u64,
    regressions: &mut Vec<String>,
) {
    if actual < expected {
        regressions.push(format!(
            "{domain:?}.{metric}: {actual} is below floor {expected}"
        ));
    }
}

fn check_maximum(
    domain: CoverageDomain,
    metric: &str,
    actual: u64,
    expected: u64,
    regressions: &mut Vec<String>,
) {
    if actual > expected {
        regressions.push(format!(
            "{domain:?}.{metric}: {actual} exceeds ceiling {expected}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_and_reason_accounting_are_checked() {
        let mut report = CoverageReport::default();
        report.categories.insert(
            CoverageDomain::Doc,
            CoverageCategoryReport {
                counts: CoverageCounts {
                    discovered: 4,
                    strict: 2,
                    compatible: 1,
                    rejected: 1,
                    round_tripped: 3,
                    ..CoverageCounts::default()
                },
                rejection_reasons: BTreeMap::from([("invalid FIB".to_owned(), 1)]),
                ..CoverageCategoryReport::default()
            },
        );
        report.validate().unwrap();
        report
            .categories
            .get_mut(&CoverageDomain::Doc)
            .unwrap()
            .counts
            .discovered = 5;
        assert!(report.validate().unwrap_err().contains("not conserved"));
    }

    #[test]
    fn every_domain_and_exclusion_reason_is_required() {
        let mut report = CoverageReport::default();
        report.categories.remove(&CoverageDomain::Forms);
        assert!(report.validate().unwrap_err().contains("Forms"));

        let mut report = CoverageReport::default();
        let doc = report.categories.get_mut(&CoverageDomain::Doc).unwrap();
        doc.counts.discovered = 1;
        doc.counts.excluded = 1;
        assert!(report.validate().unwrap_err().contains("exclusion reasons"));

        let mut ratchet =
            serde_json::from_str::<CoverageRatchet>(include_str!("../coverage-ratchet.json"))
                .unwrap();
        ratchet.categories.remove(&CoverageDomain::Forms);
        assert!(
            CoverageReport::default()
                .assert_meets(&ratchet)
                .unwrap_err()
                .contains("coverage ratchet domain inventory")
        );
    }

    #[test]
    fn ratchet_allows_coverage_growth_and_rejects_regressions() {
        let actual = CoverageCategoryReport {
            counts: CoverageCounts {
                discovered: 10,
                excluded: 1,
                strict: 6,
                compatible: 2,
                rejected: 1,
                round_tripped: 8,
            },
            metrics: BTreeMap::from([("records".to_owned(), 8)]),
            ..CoverageCategoryReport::default()
        };
        let threshold = CoverageThreshold {
            minimum_discovered: 10,
            minimum_strict: 5,
            minimum_supported: 8,
            minimum_round_tripped: 8,
            maximum_rejected: 1,
            maximum_excluded: 1,
            minimum_metrics: BTreeMap::from([("records".to_owned(), 7)]),
            maximum_metrics: BTreeMap::from([("unknown".to_owned(), 0)]),
        };
        let mut regressions = Vec::new();
        threshold.collect_regressions(CoverageDomain::Cfb, &actual, &mut regressions);
        assert!(regressions.is_empty());

        let regressed = CoverageCategoryReport {
            counts: CoverageCounts {
                strict: 4,
                rejected: 2,
                ..actual.counts
            },
            metrics: BTreeMap::from([("records".to_owned(), 6), ("unknown".to_owned(), 1)]),
            ..CoverageCategoryReport::default()
        };
        threshold.collect_regressions(CoverageDomain::Cfb, &regressed, &mut regressions);
        assert!(regressions.iter().any(|value| value.contains("strict")));
        assert!(regressions.iter().any(|value| value.contains("rejected")));
        assert!(regressions.iter().any(|value| value.contains("records")));
        assert!(regressions.iter().any(|value| value.contains("unknown")));

        let mut unratcheted = actual.clone();
        unratcheted
            .metrics
            .insert("debt.unknown_extension.type_0xffff.records".to_owned(), 1);
        threshold.collect_regressions(CoverageDomain::Cfb, &unratcheted, &mut regressions);
        assert!(
            regressions
                .iter()
                .any(|value| value.contains("no ratchet ceiling"))
        );
    }

    #[test]
    fn exhaustive_record_dispositions_and_metric_names_are_checked() {
        let mut report = CoverageReport::default();
        let xls = report.categories.get_mut(&CoverageDomain::Xls).unwrap();
        xls.metrics.insert("biff_records".to_owned(), 2);
        xls.metrics
            .insert("disposition.typed.records".to_owned(), 1);
        assert!(
            report
                .validate()
                .unwrap_err()
                .contains("disposition records")
        );

        let mut report = CoverageReport::default();
        report
            .categories
            .get_mut(&CoverageDomain::Doc)
            .unwrap()
            .metrics
            .insert("disposition.typo.records".to_owned(), 1);
        assert!(
            report
                .validate()
                .unwrap_err()
                .contains("invalid disposition metric")
        );

        let mut report = CoverageReport::default();
        let doc = report.categories.get_mut(&CoverageDomain::Doc).unwrap();
        doc.metrics.insert("content_nodes".to_owned(), 2);
        doc.metrics.insert("disposition.typed.units".to_owned(), 1);
        assert!(report.validate().unwrap_err().contains("disposition units"));

        let mut report = CoverageReport::default();
        let oleps = report
            .categories
            .get_mut(&CoverageDomain::OlePropertySet)
            .unwrap();
        oleps.metrics.insert("properties".to_owned(), 2);
        oleps.metrics.insert("property_bytes".to_owned(), 12);
        oleps
            .metrics
            .insert("disposition.typed.units".to_owned(), 1);
        oleps
            .metrics
            .insert("disposition.typed.bytes".to_owned(), 12);
        assert!(report.validate().unwrap_err().contains("disposition units"));
    }

    #[test]
    fn every_emitted_debt_family_requires_structured_evidence() {
        let catalog = bundled_coverage_evidence().unwrap();
        let ratchet =
            serde_json::from_str::<CoverageRatchet>(include_str!("../coverage-ratchet.json"))
                .unwrap();
        for (domain, evidence) in &catalog.categories {
            let maximum_metrics = &ratchet.categories.get(domain).unwrap().maximum_metrics;
            for family in evidence.keys() {
                assert!(
                    maximum_metrics
                        .keys()
                        .any(|metric| debt_metric_family(metric) == Some(family)),
                    "{domain:?}.{family} has evidence but no ratchet ceiling"
                );
            }
            for metric in maximum_metrics
                .keys()
                .filter(|metric| metric.starts_with("debt."))
            {
                let family = debt_metric_family(metric).unwrap();
                assert!(
                    evidence.contains_key(family),
                    "{domain:?}.{metric} has a debt ceiling but no evidence"
                );
            }
        }
        let mut report = CoverageReport::default();
        report
            .categories
            .get_mut(&CoverageDomain::Ppt)
            .unwrap()
            .metrics
            .insert("debt.unknown_extension.type_0x0080.records".to_owned(), 1);
        report.assert_debt_has_evidence(&catalog).unwrap();

        report
            .categories
            .get_mut(&CoverageDomain::Ppt)
            .unwrap()
            .metrics
            .insert("debt.unknown_extension.type_0xdead.records".to_owned(), 1);
        assert!(
            report
                .assert_debt_has_evidence(&catalog)
                .unwrap_err()
                .contains("debt has no evidence")
        );
    }

    #[test]
    fn json_is_stable_and_newline_terminated() {
        let report = CoverageReport::default();
        let json = report.to_pretty_json().unwrap();
        assert!(json.ends_with('\n'));
        assert_eq!(
            serde_json::from_str::<CoverageReport>(&json).unwrap(),
            report
        );
    }
}
