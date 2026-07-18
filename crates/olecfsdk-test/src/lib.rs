//! Shared coverage and regression-ratchet support for `olecfsdk` integration tests.

mod corpus;

use std::collections::BTreeMap;

pub use corpus::audit_classic_office_file_roots;
use serde::{Deserialize, Serialize};

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
    pub round_trip_failure_reasons: BTreeMap<String, u64>,
    /// Domain-specific stable counters such as records, bytes, or typed leaves.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metrics: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageReport {
    pub schema_version: u32,
    pub categories: BTreeMap<CoverageDomain, CoverageCategoryReport>,
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self {
            schema_version: 1,
            categories: BTreeMap::new(),
        }
    }
}

impl CoverageReport {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported coverage report schema version {}",
                self.schema_version
            ));
        }
        for (domain, category) in &self.categories {
            category.counts.validate(*domain)?;
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
