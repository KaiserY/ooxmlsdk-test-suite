#![doc = "LibreOffice-derived layout conformance tests for ooxmlsdk-layout."]

use std::path::{Path, PathBuf};

use ooxmlsdk::parts::{
    presentation_document::PresentationDocument, spreadsheet_document::SpreadsheetDocument,
    wordprocessing_document::WordprocessingDocument,
};
use ooxmlsdk_layout::common::{Color, DisplayItem, Fill, LayoutDocument, Point, Rect};
use ooxmlsdk_layout::{LayoutOptions, Result};

pub fn corpus_file(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/LibreOffice")
        .join(path)
}

pub fn docx_layout(path: &str) -> Result<LayoutDocument<'static>> {
    let mut package = WordprocessingDocument::new_from_file(corpus_file(path))?;
    ooxmlsdk_layout::docx::layout_document(&mut package, &LayoutOptions::default())
}

pub fn docx_layout_named(name: &str) -> Result<LayoutDocument<'static>> {
    let path = unique_corpus_file_named(name);
    let mut package = WordprocessingDocument::new_from_file(path)?;
    ooxmlsdk_layout::docx::layout_document(&mut package, &LayoutOptions::default())
}

pub fn pptx_layout(path: &str) -> Result<LayoutDocument<'static>> {
    let mut package = PresentationDocument::new_from_file(corpus_file(path))?;
    ooxmlsdk_layout::pptx::layout_document(&mut package, &LayoutOptions::default())
}

pub fn xlsx_layout(path: &str) -> Result<LayoutDocument<'static>> {
    let fixture = corpus_file(path);
    let source_file_name = fixture
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToString::to_string);
    let mut package = SpreadsheetDocument::new_from_file(fixture)?;
    ooxmlsdk_layout::xlsx::layout_document(&mut package, &LayoutOptions { source_file_name })
}

pub fn pptx_import_summary(path: &str) -> Result<ooxmlsdk_layout::pptx::PptxLayoutSummary> {
    let mut package = PresentationDocument::new_from_file(corpus_file(path))?;
    ooxmlsdk_layout::pptx::inspect_layout(&mut package)
}

pub fn assert_close(actual: f32, expected: f32, tolerance: f32, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: actual {actual}, expected {expected}, tolerance {tolerance}"
    );
}

pub fn rect_left(rect: Rect) -> f32 {
    rect.origin.x.0
}

pub fn rect_top(rect: Rect) -> f32 {
    rect.origin.y.0
}

pub fn rect_right(rect: Rect) -> f32 {
    rect.origin.x.0 + rect.size.width.0
}

pub fn rect_bottom(rect: Rect) -> f32 {
    rect.origin.y.0 + rect.size.height.0
}

pub fn point_x(point: Point) -> f32 {
    point.x.0
}

pub fn point_y(point: Point) -> f32 {
    point.y.0
}

pub fn all_page_text(document: &LayoutDocument<'_>, page_index: usize) -> String {
    document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()))
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text.text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalized_page_text(document: &LayoutDocument<'_>, page_index: usize) -> String {
    normalize_space(&all_page_text(document, page_index))
}

pub fn page_text_contains(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected: &str,
) -> bool {
    page_text_matches(&all_page_text(document, page_index), expected)
}

pub fn page_item_count(document: &LayoutDocument<'_>, page_index: usize) -> usize {
    document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()))
        .items
        .len()
}

pub fn text_origins_for<'a>(
    document: &'a LayoutDocument<'a>,
    page_index: usize,
    expected: &str,
) -> Vec<Point> {
    document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()))
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(text) if text.text.contains(expected) => Some(text.origin),
            _ => None,
        })
        .collect()
}

pub fn path_bounds(document: &LayoutDocument<'_>, page_index: usize) -> Vec<Rect> {
    document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()))
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Path(path) if path_item_is_visible(&path.fill, path.stroke.is_some()) => {
                Some(path.bounds)
            }
            DisplayItem::Rect(rect) if path_item_is_visible(&rect.fill, rect.stroke.is_some()) => {
                Some(rect.bounds)
            }
            DisplayItem::Line(line) => Some(line_rect(line.start, line.end)),
            _ => None,
        })
        .collect()
}

pub fn image_bounds(document: &LayoutDocument<'_>, page_index: usize) -> Vec<Rect> {
    document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()))
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Image(image) => Some(image.bounds),
            _ => None,
        })
        .collect()
}

pub fn line_heights(document: &LayoutDocument<'_>, page_index: usize) -> Vec<f32> {
    document
        .frames
        .iter()
        .filter(|frame| frame.page_index == page_index)
        .flat_map(|frame| frame.lines.iter().map(|line| line.bounds.size.height.0))
        .collect()
}

pub fn row_heights(document: &LayoutDocument<'_>, page_index: usize) -> Vec<f32> {
    document
        .frames
        .iter()
        .filter(|frame| frame.page_index == page_index)
        .flat_map(|frame| {
            frame.fragments.iter().filter_map(|fragment| {
                matches!(
                    fragment.kind,
                    ooxmlsdk_layout::common::FrameFragmentKind::TableRow
                )
                .then(|| fragment.bounds.map(|bounds| bounds.size.height.0))
                .flatten()
            })
        })
        .collect()
}

pub fn table_row_count_for_block(
    document: &LayoutDocument<'_>,
    page_index: usize,
    block_index: usize,
) -> usize {
    document
        .frames
        .iter()
        .filter(|frame| frame.page_index == page_index && frame.block_index == Some(block_index))
        .flat_map(|frame| &frame.fragments)
        .filter(|fragment| {
            matches!(
                fragment.kind,
                ooxmlsdk_layout::common::FrameFragmentKind::TableRow
            )
        })
        .count()
}

pub fn assert_page_contains(document: &LayoutDocument<'_>, page_index: usize, expected: &str) {
    let page_text = all_page_text(document, page_index);
    assert!(
        page_text_matches(&page_text, expected),
        "missing text {expected:?} on page {page_index}; page_text={page_text:?}"
    );
}

pub fn assert_page_contains_any(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected: &[&str],
) {
    let page_text = all_page_text(document, page_index);
    assert!(
        expected
            .iter()
            .any(|item| page_text_matches(&page_text, item)),
        "missing any text {expected:?} on page {page_index}; page_text={page_text:?}"
    );
}

pub fn assert_page_not_contains(
    document: &LayoutDocument<'_>,
    page_index: usize,
    unexpected: &str,
) {
    let page_text = all_page_text(document, page_index);
    assert!(
        !page_text_matches(&page_text, unexpected),
        "unexpected text {unexpected:?} on page {page_index}; page_text={page_text:?}"
    );
}

pub fn assert_page_text_occurrences(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected: &str,
    expected_count: usize,
) {
    let page_text = all_page_text(document, page_index);
    let normalized_page_text = normalize_space(&page_text);
    let normalized_expected = normalize_space(expected);
    let count = normalized_page_text.matches(&normalized_expected).count();
    assert_eq!(
        count, expected_count,
        "text {expected:?} occurrence mismatch on page {page_index}; page_text={page_text:?}"
    );
}

pub fn assert_page_text_occurrences_at_least(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected: &str,
    expected_count: usize,
) {
    let page_text = all_page_text(document, page_index);
    let normalized_page_text = normalize_space(&page_text);
    let normalized_expected = normalize_space(expected);
    let count = normalized_page_text.matches(&normalized_expected).count();
    assert!(
        count >= expected_count,
        "text {expected:?} occurrence mismatch on page {page_index}: expected at least {expected_count}, got {count}; page_text={page_text:?}"
    );
}

pub fn assert_page_image_count(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_count: usize,
) {
    let bounds = image_bounds(document, page_index);
    assert_eq!(
        bounds.len(),
        expected_count,
        "image count mismatch on page {page_index}; image_bounds={bounds:?}"
    );
}

pub fn assert_page_image_count_at_least(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_count: usize,
) {
    let bounds = image_bounds(document, page_index);
    assert!(
        bounds.len() >= expected_count,
        "expected at least {expected_count} images on page {page_index}; image_bounds={bounds:?}"
    );
}

pub fn assert_page_path_count_at_least(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_count: usize,
) {
    let bounds = path_bounds(document, page_index);
    assert!(
        bounds.len() >= expected_count,
        "expected at least {expected_count} paths on page {page_index}; path_bounds={bounds:?}"
    );
}

pub fn assert_page_stroked_path_count_at_least(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_count: usize,
) {
    let page = document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()));
    let count = page
        .items
        .iter()
        .filter(|item| match item {
            DisplayItem::Path(path) => path.stroke.is_some(),
            DisplayItem::Rect(rect) => rect.stroke.is_some(),
            DisplayItem::Line(_) => true,
            _ => false,
        })
        .count();
    assert!(
        count >= expected_count,
        "expected at least {expected_count} stroked paths on page {page_index}, got {count}; items={:?}",
        page.items
    );
}

pub fn assert_page_filled_path_count_at_least(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_count: usize,
) {
    let page = document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()));
    let count = page
        .items
        .iter()
        .filter(|item| match item {
            DisplayItem::Path(path) => !matches!(path.fill, ooxmlsdk_layout::common::Fill::None),
            DisplayItem::Rect(rect) => !matches!(rect.fill, ooxmlsdk_layout::common::Fill::None),
            _ => false,
        })
        .count();
    assert!(
        count >= expected_count,
        "expected at least {expected_count} filled paths on page {page_index}, got {count}; items={:?}",
        page.items
    );
}

pub fn assert_page_size(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_width: f32,
    expected_height: f32,
) {
    let page = document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()));
    assert_close(page.bounds.size.width.0, expected_width, 0.75, "page width");
    assert_close(
        page.bounds.size.height.0,
        expected_height,
        0.75,
        "page height",
    );
}

pub fn assert_link_target(document: &LayoutDocument<'_>, expected_target: &str) {
    assert!(
        link_targets(document)
            .iter()
            .any(|target| target == expected_target),
        "missing link target {expected_target:?}; links={:?}",
        link_targets(document)
    );
}

pub fn assert_text_font_size(
    document: &LayoutDocument<'_>,
    expected_text: &str,
    expected_size: f32,
) {
    let normalized_expected = normalize_space(expected_text);
    let text_runs = document
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        text_runs.iter().any(|run| {
            normalize_space(&run.text).contains(&normalized_expected)
                && (run.style.font_size.0 - expected_size).abs() <= 0.05
        }),
        "missing text {expected_text:?} with font size {expected_size}; text_runs={text_runs:?}"
    );
}

pub fn assert_text_color(
    document: &LayoutDocument<'_>,
    expected_text: &str,
    expected_color: Color,
) {
    let normalized_expected = normalize_space(expected_text);
    let text_runs = document
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .filter_map(|item| match item {
            DisplayItem::Text(text) => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        text_runs.iter().any(|run| {
            normalize_space(&run.text).contains(&normalized_expected) && run.color == expected_color
        }),
        "missing text {expected_text:?} with color {expected_color:?}; text_runs={text_runs:?}"
    );
}

pub fn assert_text_underline(document: &LayoutDocument<'_>, expected_text: &str) {
    let text_runs = text_runs_matching(document, expected_text);
    assert!(
        text_runs.iter().any(|run| run.style.underline),
        "missing underlined text {expected_text:?}; text_runs={text_runs:?}"
    );
}

pub fn assert_text_strikethrough(document: &LayoutDocument<'_>, expected_text: &str) {
    let text_runs = text_runs_matching(document, expected_text);
    assert!(
        text_runs.iter().any(|run| run.style.strikethrough),
        "missing strikethrough text {expected_text:?}; text_runs={text_runs:?}"
    );
}

pub fn assert_page_contains_in_order(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected: &[&str],
) {
    let page_text = all_page_text(document, page_index);
    let search_text = searchable_text(&page_text);
    let mut cursor = 0;
    for item in expected {
        let search_item = searchable_text(item);
        let Some(offset) = search_text[cursor..].find(&search_item) else {
            panic!(
                "missing text {item:?} after offset {cursor} on page {page_index}; page_text={page_text:?}"
            );
        };
        cursor += offset + search_item.len();
    }
}

pub fn assert_page_starts_with(document: &LayoutDocument<'_>, page_index: usize, expected: &str) {
    let page_text = all_page_text(document, page_index);
    assert!(
        searchable_text(&page_text).starts_with(&searchable_text(expected)),
        "page {page_index} does not start with {expected:?}; page_text={page_text:?}"
    );
}

pub fn assert_page_has_no_text(document: &LayoutDocument<'_>, page_index: usize) {
    let page_text = all_page_text(document, page_index);
    assert!(
        normalize_space(&page_text).is_empty(),
        "expected no text on page {page_index}; page_text={page_text:?}"
    );
}

pub fn assert_page_path_count(
    document: &LayoutDocument<'_>,
    page_index: usize,
    expected_count: usize,
) {
    let bounds = path_bounds(document, page_index);
    assert_eq!(
        bounds.len(),
        expected_count,
        "path count mismatch on page {page_index}; path_bounds={bounds:?}"
    );
}

pub fn assert_path_width_count(
    document: &LayoutDocument<'_>,
    expected_width: f32,
    expected_count: usize,
    tolerance: f32,
) {
    let bounds = document
        .pages
        .iter()
        .enumerate()
        .flat_map(|(page_index, _)| path_bounds(document, page_index))
        .collect::<Vec<_>>();
    let actual_count = bounds
        .iter()
        .filter(|bounds| (bounds.size.width.0 - expected_width).abs() <= tolerance)
        .count();
    assert_eq!(
        actual_count, expected_count,
        "path width {expected_width}pt count mismatch; path_bounds={bounds:?}"
    );
}

fn link_targets(document: &LayoutDocument<'_>) -> Vec<String> {
    document
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .filter_map(|item| match item {
            DisplayItem::LinkArea(link) => Some(link.target.to_string()),
            DisplayItem::Text(text) => text.hyperlink_url.as_ref().map(ToString::to_string),
            DisplayItem::Image(image) => image.hyperlink_url.as_ref().map(ToString::to_string),
            _ => None,
        })
        .collect()
}

fn text_runs_matching<'a, 'doc>(
    document: &'a LayoutDocument<'doc>,
    expected_text: &str,
) -> Vec<&'a ooxmlsdk_layout::common::TextRun<'doc>> {
    let normalized_expected = normalize_space(expected_text);
    document
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .filter_map(|item| match item {
            DisplayItem::Text(text)
                if page_text_matches(text.text.as_ref(), &normalized_expected) =>
            {
                Some(text)
            }
            _ => None,
        })
        .collect()
}

fn path_item_is_visible(fill: &Fill<'_>, has_stroke: bool) -> bool {
    !matches!(fill, Fill::None) || has_stroke
}

fn page_text_matches(page_text: &str, expected: &str) -> bool {
    let normalized_page_text = normalize_space(page_text);
    let normalized_expected = normalize_space(expected);
    normalized_page_text.contains(&normalized_expected)
        || searchable_text(page_text).contains(&searchable_text(expected))
}

fn searchable_text(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn unique_corpus_file_named(name: &str) -> PathBuf {
    let root = corpus_file("");
    let mut matches = Vec::new();
    find_corpus_file_named(&root, name, &mut matches);
    match matches.len() {
        1 => matches.remove(0),
        0 => panic!("missing LibreOffice corpus fixture named {name:?} under {root:?}"),
        _ => panic!("ambiguous LibreOffice corpus fixture named {name:?}: {matches:?}"),
    }
}

fn find_corpus_file_named(dir: &Path, name: &str, matches: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_corpus_file_named(&path, name, matches);
        } else if path.file_name().and_then(|file_name| file_name.to_str()) == Some(name) {
            matches.push(path);
        }
    }
}

fn line_rect(start: Point, end: Point) -> Rect {
    let left = point_x(start).min(point_x(end));
    let top = point_y(start).min(point_y(end));
    let right = point_x(start).max(point_x(end));
    let bottom = point_y(start).max(point_y(end));
    Rect {
        origin: Point {
            x: ooxmlsdk_layout::common::Pt(left),
            y: ooxmlsdk_layout::common::Pt(top),
        },
        size: ooxmlsdk_layout::common::Size {
            width: ooxmlsdk_layout::common::Pt(right - left),
            height: ooxmlsdk_layout::common::Pt(bottom - top),
        },
    }
}
