use ooxmlsdk_corpus_test_support::{
    corpus_file_path,
    roundtrip::{
        assert_package_file_invalid, assert_package_file_opens, assert_package_file_round_trip,
    },
};

fn assert_corpus_round_trip(relative_path: &str) {
    assert_package_file_round_trip(&corpus_file_path(relative_path), relative_path);
}

#[allow(dead_code)]
fn assert_corpus_opens(relative_path: &str) {
    assert_package_file_opens(&corpus_file_path(relative_path), relative_path);
}

#[allow(dead_code)]
fn assert_corpus_invalid(relative_path: &str) {
    assert_package_file_invalid(&corpus_file_path(relative_path), relative_path);
}

include!(concat!(env!("OUT_DIR"), "/open_xml_sdk_roundtrip_tests.rs"));
