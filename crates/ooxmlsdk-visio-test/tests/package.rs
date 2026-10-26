use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use ooxmlsdk::common::resolve_relationship_target_path;
use ooxmlsdk::parts::visio_document::VisioDocument;
use ooxmlsdk::sdk::VisioDocumentType;
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

#[test]
fn minimal_drawing_exports_namespace_valid_xml() {
    let package = minimal_drawing(&mut Ids::new()).unwrap();
    assert_namespace_valid(&xml_parts(&package), 3);
}

#[test]
fn minimal_drawing_page_rel_resolves_to_page_contents() {
    let package = minimal_drawing(&mut Ids::new()).unwrap();
    assert_page_relationships(&xml_parts(&package), &[1]);
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
    add_color(&mut reopened, 2, "#FFFFFF").unwrap();
    add_page(&mut reopened, &mut ids, Some("Third"), "8", "10").unwrap();
    add_master(&mut reopened, &mut ids, Some("Second master")).unwrap();
    add_test_author(&mut reopened, 2);
    let parts = xml_parts(&reopened);
    assert_namespace_valid(&parts, 9);
    assert_page_relationships(&parts, &[1, 2, 3]);
}
