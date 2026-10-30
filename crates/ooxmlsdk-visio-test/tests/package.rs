use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};

use ooxmlsdk::common::{SdkError, resolve_relationship_target_path};
use ooxmlsdk::parts::visio_comments_part::VisioCommentsPart;
use ooxmlsdk::parts::visio_document::VisioDocument;
use ooxmlsdk::parts::visio_pages_part::VisioPagesPart;
use ooxmlsdk::sdk::{SdkPart, VisioDocumentType};
use ooxmlsdk_visio::{
    AuthorRecord, Ids, add_author, add_color, add_master, add_page, minimal_drawing,
};
use roxmltree::Document;

const VISIO: &str = "http://schemas.microsoft.com/office/visio/2012/main";
const RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const PAGE: &str = "http://schemas.microsoft.com/visio/2010/relationships/page";

fn xml_parts(package: &VisioDocument) -> BTreeMap<String, String> {
    let bytes = package.to_package_bytes().expect("save drawing");
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("open ZIP");
    let mut parts = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().expect("ZIP entry name").into_owned();
        if name.ends_with(".xml") || name.ends_with(".rels") {
            let mut xml = String::new();
            entry.read_to_string(&mut xml).unwrap();
            parts.insert(name, xml);
        }
    }
    parts
}

fn assert_namespace_valid(parts: &BTreeMap<String, String>, expected_visio_parts: usize) {
    let mut visio_parts = 0;
    for (name, xml) in parts {
        let document = Document::parse(xml).unwrap_or_else(|error| panic!("{name}: {error}"));
        if name.starts_with("visio/") && name.ends_with(".xml") {
            visio_parts += 1;
            assert_eq!(
                document.root_element().tag_name().namespace(),
                Some(VISIO),
                "{name}"
            );
        }
    }
    assert_eq!(visio_parts, expected_visio_parts);
}

fn assert_page_relationships(parts: &BTreeMap<String, String>, expected_ids: &[u32]) {
    let (name, xml) = parts
        .iter()
        .find(|(name, _)| name.ends_with("/pages.xml"))
        .expect("pages part");
    let (directory, filename) = name.rsplit_once('/').unwrap();
    let relationship_part = format!("{directory}/_rels/{filename}.rels");
    let relationships = Document::parse(&parts[&relationship_part]).unwrap();
    let pages = Document::parse(xml).unwrap();
    let mut targets = BTreeSet::new();
    let mut ids = Vec::new();
    for page in pages
        .root_element()
        .children()
        .filter(|node| node.has_tag_name((VISIO, "Page")))
    {
        ids.push(page.attribute("ID").unwrap().parse::<u32>().unwrap());
        let rel = page
            .children()
            .find(|node| node.has_tag_name((VISIO, "Rel")))
            .expect("page Rel");
        let id = rel
            .attribute((RELATIONSHIPS, "id"))
            .expect("page Rel/@r:id");
        assert!(!id.is_empty());
        let relationship = relationships
            .root_element()
            .children()
            .find(|node| {
                node.has_tag_name((PACKAGE_RELATIONSHIPS, "Relationship"))
                    && node.attribute("Id") == Some(id)
            })
            .expect("page relationship exists");
        assert_eq!(relationship.attribute("Type"), Some(PAGE));
        assert_ne!(relationship.attribute("TargetMode"), Some("External"));
        let target = resolve_relationship_target_path(
            &format!("{directory}/"),
            relationship.attribute("Target").unwrap(),
        );
        assert!(
            targets.insert(target.clone()),
            "pages share target {target}"
        );
        let contents = Document::parse(parts.get(&target).expect("page target exists")).unwrap();
        assert!(
            contents
                .root_element()
                .has_tag_name((VISIO, "PageContents")),
            "{target}"
        );
    }
    assert_eq!(ids, expected_ids);
}

fn add_test_author(package: &mut VisioDocument, id: u32) {
    add_author(
        package,
        AuthorRecord {
            id,
            name: Some(format!("Author {id}")),
            initials: None,
            sip: None,
            smtp: None,
            resolution_id: None,
        },
    )
    .unwrap();
}

fn authored_drawing() -> (VisioDocument, Ids) {
    let mut package = VisioDocument::create(VisioDocumentType::Drawing);
    package.add_visio_document_part().unwrap();
    let mut ids = Ids::new();
    add_color(&mut package, 1, "#000000").unwrap();
    add_page(&mut package, &mut ids, Some("First"), "8", "10").unwrap();
    add_page(&mut package, &mut ids, Some("Second"), "8", "10").unwrap();
    add_master(&mut package, &mut ids, Some("Master")).unwrap();
    add_test_author(&mut package, 1);
    (package, ids)
}

fn edit_and_validate_drawing(
    package: &mut VisioDocument,
    ids: &mut Ids,
) -> BTreeMap<String, String> {
    add_color(package, 2, "#FFFFFF").unwrap();
    add_page(package, ids, Some("Third"), "8", "10").unwrap();
    add_master(package, ids, Some("Second master")).unwrap();
    add_test_author(package, 2);
    let parts = xml_parts(package);
    assert_namespace_valid(&parts, 9);
    assert_page_relationships(&parts, &[1, 2, 3]);
    parts
}

#[test]
fn minimal_drawing_exports_namespace_valid_xml() {
    let package = minimal_drawing(&mut Ids::new()).unwrap();
    assert_namespace_valid(&xml_parts(&package), 3);
}

#[test]
fn editing_document_preserves_unrelated_relationship_prefix() {
    let mut package = minimal_drawing(&mut Ids::new()).unwrap();
    let part = package.visio_document_part().unwrap();
    part.set_data(
        &mut package,
        format!(r#"<visio:VisioDocument xmlns:visio="{VISIO}" xmlns:r="urn:customer-extension"/>"#)
            .into_bytes(),
    )
    .unwrap();
    add_color(&mut package, 1, "#000000").unwrap();
    let parts = xml_parts(&package);
    assert_namespace_valid(&parts, 3);
    let (_, xml) = parts
        .iter()
        .find(|(name, _)| name.ends_with("/document.xml"))
        .unwrap();
    let document = Document::parse(xml).unwrap();
    assert_eq!(
        document.root_element().lookup_namespace_uri(Some("r")),
        Some("urn:customer-extension")
    );
}

#[test]
fn editing_comments_preserves_unrelated_relationship_prefix() {
    let (mut package, _) = authored_drawing();
    let document = package.visio_document_part().unwrap();
    let comments = document
        .child_part_by_relationship_type::<_, VisioCommentsPart>(
            &package,
            "http://schemas.microsoft.com/visio/2010/relationships/comments",
        )
        .unwrap();
    comments
        .set_data(
            &mut package,
            format!(r#"<visio:Comments xmlns:visio="{VISIO}" xmlns:r="urn:customer-extension"/>"#)
                .into_bytes(),
        )
        .unwrap();
    add_test_author(&mut package, 2);
    let parts = xml_parts(&package);
    assert_namespace_valid(&parts, 7);
    let (_, xml) = parts
        .iter()
        .find(|(name, _)| name.ends_with("/comments.xml"))
        .unwrap();
    let comments = Document::parse(xml).unwrap();
    assert_eq!(
        comments.root_element().lookup_namespace_uri(Some("r")),
        Some("urn:customer-extension")
    );
}

#[test]
fn adding_page_rejects_relationship_prefix_collision_before_mutation() {
    let mut ids = Ids::new();
    let mut package = minimal_drawing(&mut ids).unwrap();
    let document = package.visio_document_part().unwrap();
    let pages = document
        .child_part_by_relationship_type::<_, VisioPagesPart>(
            &package,
            "http://schemas.microsoft.com/visio/2010/relationships/pages",
        )
        .unwrap();
    pages
        .set_data(
            &mut package,
            format!(r#"<visio:Pages xmlns:visio="{VISIO}" xmlns:r="urn:customer-extension"/>"#)
                .into_bytes(),
        )
        .unwrap();
    let before = xml_parts(&package);
    assert_namespace_valid(&before, 3);
    let error = add_page(&mut package, &mut ids, Some("Next"), "8", "10").unwrap_err();
    assert!(
        matches!(&error, SdkError::CommonError(message) if message.contains("namespace prefix 'r'")),
        "{error:?}"
    );
    assert_eq!(xml_parts(&package), before);
    assert_eq!(ids.page(), 2);
}

#[test]
fn minimal_drawing_page_rel_resolves_to_page_contents() {
    let mut ids = Ids::new();
    for _ in 0..16 {
        ids.page();
    }
    let mut package = minimal_drawing(&mut ids).unwrap();
    assert_page_relationships(&xml_parts(&package), &[17]);
    add_page(&mut package, &mut ids, Some("Next"), "8", "10").unwrap();
    let parts = xml_parts(&package);
    assert_namespace_valid(&parts, 4);
    assert_page_relationships(&parts, &[17, 18]);
}

#[test]
fn fresh_pages_masters_and_comments_have_namespace_bindings() {
    let (package, _) = authored_drawing();
    let parts = xml_parts(&package);
    assert_namespace_valid(&parts, 7);
    assert_page_relationships(&parts, &[1, 2]);
}

#[test]
fn editing_reopened_roots_keeps_namespace_bindings_unique() {
    let (package, mut ids) = authored_drawing();
    let bytes = package.to_package_bytes().unwrap();
    let mut reopened = VisioDocument::new(Cursor::new(bytes)).unwrap();
    edit_and_validate_drawing(&mut reopened, &mut ids);
}

#[test]
fn editing_imported_roots_preserves_namespace_bindings() {
    for (visio_prefix, relationship_prefix, encoded) in [("", "rel", false), ("v", "rel", true)] {
        let (package, mut ids) = authored_drawing();
        let mut imported_parts = xml_parts(&package);
        let mut imported_visio_parts = Vec::new();
        for (name, xml) in &mut imported_parts {
            if !name.starts_with("visio/") || !name.ends_with(".xml") {
                continue;
            }
            let has_relationship_namespace = Document::parse(xml)
                .unwrap()
                .root_element()
                .lookup_namespace_uri(Some("r"))
                .is_some();
            *xml = xml.replace(
                " xmlns:visio=",
                " xmlns:extra=\"urn:test-extra\" xmlns:visio=",
            );
            let (declaration, prefix) = if visio_prefix.is_empty() {
                ("xmlns=".to_string(), String::new())
            } else {
                (format!("xmlns:{visio_prefix}="), format!("{visio_prefix}:"))
            };
            *xml = xml
                .replace("xmlns:visio=", &declaration)
                .replace("<visio:", &format!("<{prefix}"))
                .replace("</visio:", &format!("</{prefix}"));
            *xml = xml
                .replace("xmlns:r=", &format!("xmlns:{relationship_prefix}="))
                .replace(" r:", &format!(" {relationship_prefix}:"));
            if encoded {
                *xml = xml
                    .replace(
                        VISIO,
                        "http://schemas.microsoft.com/office/visio/2012/ma&#x69;n",
                    )
                    .replace(
                        RELATIONSHIPS,
                        "http://schemas.openxmlformats.org/officeDocument/2006/relationsh&#x69;ps",
                    );
            }
            let document = Document::parse(xml).unwrap();
            let root = document.root_element();
            let prefix = (!visio_prefix.is_empty()).then_some(visio_prefix);
            assert_eq!(root.lookup_namespace_uri(prefix), Some(VISIO));
            assert_eq!(
                root.lookup_namespace_uri(Some(relationship_prefix)),
                has_relationship_namespace.then_some(RELATIONSHIPS)
            );
            assert_eq!(root.lookup_namespace_uri(Some("visio")), None);
            assert_eq!(root.lookup_namespace_uri(Some("r")), None);
            imported_visio_parts.push(name.clone());
        }
        assert_namespace_valid(&imported_parts, 7);
        assert_page_relationships(&imported_parts, &[1, 2]);
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, xml) in &imported_parts {
            archive
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(xml.as_bytes()).unwrap();
        }
        let bytes = archive.finish().unwrap().into_inner();
        let mut reopened = VisioDocument::new(Cursor::new(bytes)).unwrap();
        let parts = edit_and_validate_drawing(&mut reopened, &mut ids);
        for name in imported_visio_parts {
            let document = Document::parse(&parts[&name]).unwrap();
            assert_eq!(
                document.root_element().lookup_namespace_uri(Some("extra")),
                Some("urn:test-extra"),
                "{name}"
            );
        }
    }
}
