use std::collections::BTreeMap;

use emfsdk::emfplus::EmfPlusRecordData;
use emfsdk::{EmfRecordData, EmrComment, Metafile, WmfRecordData};
use emfsdk_test::{collect_metafiles, corpus_bytes, corpus_dir, expects_parse_rejected};

fn main() {
    let root = corpus_dir("");
    let mut failures = BTreeMap::<String, Failure>::new();
    let mut rejected_files = 0usize;

    for path in collect_metafiles(&root) {
        if expects_parse_rejected(&path) {
            continue;
        }
        let sample = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let Ok(bytes) = corpus_bytes(&path) else {
            rejected_files += 1;
            continue;
        };
        let Ok(metafile) = Metafile::from_bytes(&bytes) else {
            rejected_files += 1;
            continue;
        };

        match metafile {
            Metafile::Emf(value) => {
                for (record_index, record) in value.records.into_iter().enumerate() {
                    let record_sample = format!(
                        "{sample} record={record_index} data_len={} prefix={:02X?}",
                        record.data.len(),
                        &record.data[..record.data.len().min(64)]
                    );
                    match record.parse_data() {
                        Ok(EmfRecordData::Unknown(_)) => increment(
                            &mut failures,
                            &record_sample,
                            format!(
                                "EMF {:#010X} {:?}: unknown record",
                                record.record_type,
                                record.record_kind()
                            ),
                        ),
                        Ok(data) => {
                            if let EmfRecordData::Comment(EmrComment::EmfPlus { records, .. }) =
                                &data
                            {
                                for nested_record in records {
                                    match nested_record.parse_data() {
                                        Ok(EmfPlusRecordData::Unknown(_)) => increment(
                                            &mut failures,
                                            &record_sample,
                                            format!(
                                                "EMF+ {:#06X} {:?}: unknown record",
                                                nested_record.record_type,
                                                nested_record.record_kind()
                                            ),
                                        ),
                                        Ok(data) => {
                                            match emfsdk::emfplus::EmfPlusRecord::from_data(
                                                &data,
                                                nested_record.flags(),
                                            ) {
                                                Ok(mut rebuilt) => {
                                                    rebuilt
                                                        .padding
                                                        .clone_from(&nested_record.padding);
                                                    if rebuilt != *nested_record {
                                                        increment(
                                                            &mut failures,
                                                            &record_sample,
                                                            format!(
                                                                "EMF+ {:#06X} {:?}: typed rebuild differs",
                                                                nested_record.record_type,
                                                                nested_record.record_kind()
                                                            ),
                                                        );
                                                    }
                                                }
                                                Err(error) => increment(
                                                    &mut failures,
                                                    &record_sample,
                                                    format!(
                                                        "EMF+ {:#06X} {:?}: typed write: {error}",
                                                        nested_record.record_type,
                                                        nested_record.record_kind()
                                                    ),
                                                ),
                                            }
                                        }
                                        Err(error) => increment(
                                            &mut failures,
                                            &record_sample,
                                            format!(
                                                "EMF+ {:#06X} {:?} object={:?}: {error}",
                                                nested_record.record_type,
                                                nested_record.record_kind(),
                                                nested_record.flags().object_type(),
                                            ),
                                        ),
                                    }
                                }
                            }
                            match data.to_record() {
                                Ok(rebuilt) if rebuilt == record => {}
                                Ok(_) => increment(
                                    &mut failures,
                                    &record_sample,
                                    format!(
                                        "EMF {:#010X} {:?}: typed rebuild differs",
                                        record.record_type,
                                        record.record_kind()
                                    ),
                                ),
                                Err(error) => increment(
                                    &mut failures,
                                    &record_sample,
                                    format!(
                                        "EMF {:#010X} {:?}: typed write: {error}",
                                        record.record_type,
                                        record.record_kind()
                                    ),
                                ),
                            }
                        }
                        Err(error) => increment(
                            &mut failures,
                            &record_sample,
                            format!(
                                "EMF {:#010X} {:?}: {error}",
                                record.record_type,
                                record.record_kind()
                            ),
                        ),
                    }
                }
            }
            Metafile::Wmf(value) => {
                for (record_index, record) in value.records.into_iter().enumerate() {
                    let record_sample = format!(
                        "{sample} record={record_index} data_len={} prefix={:02X?}",
                        record.data.len(),
                        &record.data[..record.data.len().min(64)]
                    );
                    match record.parse_data() {
                        Ok(WmfRecordData::Unknown(_)) => increment(
                            &mut failures,
                            &record_sample,
                            format!(
                                "WMF {:#06X} {:?}: unknown record",
                                record.function,
                                record.normalized_function_kind()
                            ),
                        ),
                        Ok(data) => match data.to_record_with_function(record.function) {
                            Ok(rebuilt) if rebuilt == record => {}
                            Ok(_) => increment(
                                &mut failures,
                                &record_sample,
                                format!(
                                    "WMF {:#06X} {:?}: typed rebuild differs",
                                    record.function,
                                    record.normalized_function_kind()
                                ),
                            ),
                            Err(error) => increment(
                                &mut failures,
                                &record_sample,
                                format!(
                                    "WMF {:#06X} {:?}: typed write: {error}",
                                    record.function,
                                    record.normalized_function_kind()
                                ),
                            ),
                        },
                        Err(error) => increment(
                            &mut failures,
                            &record_sample,
                            format!(
                                "WMF {:#06X} {:?}: {error}",
                                record.function,
                                record.normalized_function_kind()
                            ),
                        ),
                    }
                }
            }
        }
    }

    let mut failures: Vec<_> = failures.into_iter().collect();
    failures.sort_by(|left, right| {
        right
            .1
            .count
            .cmp(&left.1.count)
            .then_with(|| left.0.cmp(&right.0))
    });

    println!("unexpected file failures: {rejected_files}");
    for (failure, profile) in failures {
        println!(
            "{:>8}  {failure}\n          sample: {}",
            profile.count, profile.sample
        );
    }
}

#[derive(Debug)]
struct Failure {
    count: usize,
    sample: String,
}

fn increment(counts: &mut BTreeMap<String, Failure>, sample: &str, key: String) {
    let failure = counts.entry(key).or_insert_with(|| Failure {
        count: 0,
        sample: sample.to_string(),
    });
    failure.count += 1;
}
