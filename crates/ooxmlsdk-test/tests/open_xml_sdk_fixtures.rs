use std::path::{Path, PathBuf};

use ooxmlsdk::parts::{PartRef, wordprocessing_document::WordprocessingDocument};
use ooxmlsdk::sdk::CustomXmlPartType;
#[cfg(feature = "flat-opc")]
use std::io::Cursor;

fn suite_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate should live under <suite>/crates/<crate>")
        .to_path_buf()
}

fn open_xml_sdk_corpus_test_file(path: &str) -> PathBuf {
    let path = suite_root()
        .join("corpus/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles")
        .join(path);
    assert!(
        path.is_file(),
        "missing Open-XML-SDK corpus file: {}",
        path.display()
    );
    path
}

fn open_xml_sdk_fixture_test_file(path: &str) -> PathBuf {
    let path = suite_root()
        .join("fixtures/Open-XML-SDK/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles")
        .join(path);
    assert!(
        path.is_file(),
        "missing Open-XML-SDK fixture file: {}",
        path.display()
    );
    path
}

#[cfg(feature = "flat-opc")]
fn package_entry_names(bytes: Vec<u8>) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut names = Vec::new();
    for index in 0..archive.len() {
        names.push(archive.by_index(index).unwrap().name().to_string());
    }
    names.sort();
    names
}

#[cfg(feature = "flat-opc")]
fn package_entries(package: &WordprocessingDocument) -> Vec<String> {
    package_entry_names(package.to_package_bytes().unwrap())
}

fn custom_xml_part_data(package: &WordprocessingDocument) -> Vec<Vec<u8>> {
    let main_part = package.main_document_part().unwrap();
    main_part
        .parts(package)
        .filter_map(|part| match part.part {
            PartRef::CustomXmlPart(custom_xml_part) => {
                custom_xml_part.data(package).map(<[u8]>::to_vec)
            }
            _ => None,
        })
        .collect()
}

#[test]
#[cfg(feature = "flat-opc")]
fn flat_opc_fixture_materializes_same_parts_as_hello_world_docx() {
    // Source: ../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests/Documents/FlatOpcAndCloningTests.cs
    //   DocumentsHaveIdenticalParts
    let docx_package =
        WordprocessingDocument::new_from_file(open_xml_sdk_corpus_test_file("HelloWorld.docx"))
            .unwrap();

    let flat_opc =
        std::fs::read_to_string(open_xml_sdk_fixture_test_file("HelloWorldFlatOpc.xml")).unwrap();
    let flat_opc_package = WordprocessingDocument::from_flat_opc_str(&flat_opc).unwrap();

    assert_eq!(
        package_entries(&docx_package),
        package_entries(&flat_opc_package)
    );
}

#[test]
#[cfg(feature = "flat-opc")]
fn flat_opc_fixture_package_can_be_cloned() {
    // Source: ../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests/Documents/FlatOpcAndCloningTests.cs
    //   CanCloneDocxDocument
    //   CanCloneFlatOpcDocument
    let docx_package =
        WordprocessingDocument::new_from_file(open_xml_sdk_corpus_test_file("HelloWorld.docx"))
            .unwrap();
    let docx_clone = docx_package.to_owned_package().unwrap();
    assert_eq!(package_entries(&docx_package), package_entries(&docx_clone));

    let flat_opc =
        std::fs::read_to_string(open_xml_sdk_fixture_test_file("HelloWorldFlatOpc.xml")).unwrap();
    let flat_opc_package = WordprocessingDocument::from_flat_opc_str(&flat_opc).unwrap();
    let flat_opc_clone = flat_opc_package.to_owned_package().unwrap();
    assert_eq!(
        package_entries(&flat_opc_package),
        package_entries(&flat_opc_clone)
    );
}

#[test]
fn clone_flush_preserves_added_custom_xml_fixture_part() {
    // Source: ../Open-XML-SDK/test/DocumentFormat.OpenXml.Tests/SaveAndCloneTests.cs
    //   CanWildlyCloneAndFlush
    let source =
        WordprocessingDocument::new_from_file(open_xml_sdk_corpus_test_file("Document.docx"))
            .unwrap();
    let custom_xml = std::fs::read(open_xml_sdk_fixture_test_file("DocProperties.xml")).unwrap();

    for _ in 0..10 {
        let mut clone = source.to_owned_package().unwrap();

        let second_clone = clone.to_owned_package().unwrap();
        assert!(second_clone.to_package_bytes().is_ok());

        let main_part = clone.main_document_part().unwrap();
        let custom_part = main_part
            .add_custom_xml_part_by_type(&mut clone, CustomXmlPartType::CustomXml)
            .unwrap();
        custom_part
            .set_data(&mut clone, custom_xml.clone())
            .unwrap();

        let saved = clone.to_package_bytes().unwrap();
        let reopened = WordprocessingDocument::new(std::io::Cursor::new(saved)).unwrap();
        assert!(
            custom_xml_part_data(&reopened)
                .iter()
                .any(|data| data == &custom_xml),
            "added custom XML fixture part should survive clone/save/reopen"
        );

        let third_clone = clone.to_owned_package().unwrap();
        assert!(third_clone.to_package_bytes().is_ok());
    }
}
