#![doc = "LibreOffice-derived layout conformance tests for ooxmlsdk-layout."]

use std::{
    any::Any,
    path::{Path, PathBuf},
    sync::Mutex,
};

use ooxmlsdk::parts::{
    presentation_document::PresentationDocument, spreadsheet_document::SpreadsheetDocument,
    wordprocessing_document::WordprocessingDocument,
};
use ooxmlsdk::sdk::{
    FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings,
    OpenSettings,
};
use ooxmlsdk_layout::common::{
    Color, DebugRecord, DebugShape, DebugValue, DisplayItem, Fill, FrameFragmentKind,
    LayoutDocument, Point, Rect,
};
use ooxmlsdk_layout::options::LayoutDiagnosticsOptions;
use ooxmlsdk_layout::{LayoutOptions, Result};

const DEFAULT_LAYOUT_CASE_WORKERS: usize = 4;

pub fn corpus_file(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/LibreOffice")
        .join(path)
}

fn office_open_settings() -> OpenSettings {
    OpenSettings {
        markup_compatibility_process_settings: MarkupCompatibilityProcessSettings {
            process_mode: MarkupCompatibilityProcessMode::ProcessLoadedPartsOnly,
            target_file_format_version: FileFormatVersion::Microsoft365,
        },
        ..Default::default()
    }
}

pub fn docx_layout(path: &str) -> Result<LayoutDocument<'static>> {
    let file = std::fs::File::open(corpus_file(path)).map_err(ooxmlsdk::common::SdkError::from)?;
    let mut package = WordprocessingDocument::new_with_settings(file, office_open_settings())?;
    ooxmlsdk_layout::docx::layout_document(&mut package, &LayoutOptions::default())
}

pub fn docx_layout_named(name: &str) -> Result<LayoutDocument<'static>> {
    let path = unique_corpus_file_named(name);
    let file = std::fs::File::open(path).map_err(ooxmlsdk::common::SdkError::from)?;
    let mut package = WordprocessingDocument::new_with_settings(file, office_open_settings())?;
    ooxmlsdk_layout::docx::layout_document(&mut package, &LayoutOptions::default())
}

pub fn pptx_layout(path: &str) -> Result<LayoutDocument<'static>> {
    let file = std::fs::File::open(corpus_file(path)).map_err(ooxmlsdk::common::SdkError::from)?;
    let mut package = PresentationDocument::new_with_settings(file, office_open_settings())?;
    ooxmlsdk_layout::pptx::layout_document(
        &mut package,
        &LayoutOptions {
            diagnostics: LayoutDiagnosticsOptions {
                collect_debug_records: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}

pub fn xlsx_layout(path: &str) -> Result<LayoutDocument<'static>> {
    let fixture = corpus_file(path);
    let source_file_name = fixture
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToString::to_string);
    let file = std::fs::File::open(fixture).map_err(ooxmlsdk::common::SdkError::from)?;
    let mut package = SpreadsheetDocument::new_with_settings(file, office_open_settings())?;
    ooxmlsdk_layout::xlsx::layout_document(
        &mut package,
        &LayoutOptions {
            source_file_name,
            diagnostics: LayoutDiagnosticsOptions {
                collect_debug_records: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}

pub fn assert_close(actual: f32, expected: f32, tolerance: f32, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: actual {actual}, expected {expected}, tolerance {tolerance}"
    );
}

pub fn debug_shapes<'a, 'doc>(
    document: &'a LayoutDocument<'doc>,
    kind: &str,
) -> Vec<&'a DebugShape<'doc>> {
    document
        .debug_records
        .iter()
        .filter_map(|record| match record {
            DebugRecord::Shape(shape) if shape.kind == kind => Some(shape),
            _ => None,
        })
        .collect()
}

pub fn debug_text_property<'a, 'doc>(shape: &'a DebugShape<'doc>, name: &str) -> Option<&'a str> {
    shape.metadata.iter().find_map(|property| {
        (property.name == name)
            .then_some(&property.value)
            .and_then(|value| match value {
                DebugValue::Text(text) => Some(text.as_ref()),
                _ => None,
            })
    })
}

pub fn debug_integer_property(shape: &DebugShape<'_>, name: &str) -> Option<i64> {
    shape.metadata.iter().find_map(|property| {
        (property.name == name)
            .then_some(&property.value)
            .and_then(|value| match value {
                DebugValue::Integer(value) => Some(*value),
                _ => None,
            })
    })
}

pub fn debug_bool_property(shape: &DebugShape<'_>, name: &str) -> Option<bool> {
    shape.metadata.iter().find_map(|property| {
        (property.name == name)
            .then_some(&property.value)
            .and_then(|value| match value {
                DebugValue::Bool(value) => Some(*value),
                _ => None,
            })
    })
}

pub fn debug_shape_has_text_property(shape: &DebugShape<'_>, name: &str, expected: &str) -> bool {
    debug_text_property(shape, name).is_some_and(|value| value.contains(expected))
}

pub fn debug_shape_integer_close(shape: &DebugShape<'_>, name: &str, expected: i64) -> bool {
    debug_integer_property(shape, name).is_some_and(|value| (value - expected).abs() <= 3)
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
    let page = document
        .pages
        .get(page_index)
        .unwrap_or_else(|| panic!("missing page {page_index}; pages={}", document.pages.len()));
    let text_capacity = page
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text(run) => Some(run.text.len() + 1),
            _ => None,
        })
        .sum::<usize>()
        .saturating_sub(1);
    let mut text = String::with_capacity(text_capacity);
    for item in &page.items {
        let DisplayItem::Text(run) = item else {
            continue;
        };
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(run.text.as_ref());
    }
    text
}

pub fn normalize_space(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    for part in text.split_whitespace() {
        if !normalized.is_empty() {
            normalized.push(' ');
        }
        normalized.push_str(part);
    }
    normalized
}

pub fn run_cases_parallel<T, Run, Describe>(
    cases: &'static [T],
    run: Run,
    describe: Describe,
) -> Vec<String>
where
    T: Sync + 'static,
    Run: Fn(&T) + Send + Sync + Copy,
    Describe: Fn(&T, String) -> String + Send + Sync + Copy,
{
    run_selected_cases_parallel(cases.iter().enumerate().collect(), run, describe)
}

pub fn run_named_cases_parallel<T, Name, Run, Describe>(
    cases: &'static [T],
    name: Name,
    run: Run,
    describe: Describe,
) -> Vec<String>
where
    T: Sync + 'static,
    Name: Fn(&T) -> &str,
    Run: Fn(&T) + Send + Sync + Copy,
    Describe: Fn(&T, String) -> String + Send + Sync + Copy,
{
    let requested = std::env::var("OOXMLSDK_LAYOUT_CASE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let selected = cases
        .iter()
        .enumerate()
        .filter(|(_, case)| {
            requested
                .as_deref()
                .is_none_or(|requested| name(case) == requested)
        })
        .collect::<Vec<_>>();
    if selected.is_empty()
        && let Some(requested) = requested
    {
        return vec![format!(
            "unknown OOXMLSDK_LAYOUT_CASE={requested:?}; available cases: {}",
            cases.iter().map(&name).collect::<Vec<_>>().join(", ")
        )];
    }
    run_selected_cases_parallel(selected, run, describe)
}

fn run_selected_cases_parallel<T, Run, Describe>(
    cases: Vec<(usize, &'static T)>,
    run: Run,
    describe: Describe,
) -> Vec<String>
where
    T: Sync + 'static,
    Run: Fn(&T) + Send + Sync + Copy,
    Describe: Fn(&T, String) -> String + Send + Sync + Copy,
{
    if cases.is_empty() {
        return Vec::new();
    }
    let worker_limit = std::env::var("OOXMLSDK_LAYOUT_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|workers| *workers > 0)
        .unwrap_or(DEFAULT_LAYOUT_CASE_WORKERS);
    let worker_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(worker_limit)
        .min(cases.len());
    let chunk_size = cases.len().div_ceil(worker_count);
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for chunk in cases.chunks(chunk_size) {
            let failures = &failures;
            scope.spawn(move || {
                for (case_index, case) in chunk {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        run(case);
                    }));
                    if let Err(error) = result {
                        failures
                            .lock()
                            .unwrap()
                            .push((*case_index, describe(case, panic_message(error))));
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort_by_key(|(case_index, _)| *case_index);
    failures.into_iter().map(|(_, failure)| failure).collect()
}

fn panic_message(error: Box<dyn Any + Send>) -> String {
    if let Some(message) = error.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = error.downcast_ref::<&str>() {
        (*message).to_string()
    } else {
        "unknown panic".to_string()
    }
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

pub fn assert_image_below_table_top_and_flush_right(
    document: &LayoutDocument<'_>,
    page_index: usize,
    tolerance: f32,
) {
    let images = image_bounds(document, page_index);
    let rows = table_row_fragment_bounds(document, page_index);
    assert!(
        images.iter().any(|image| {
            rows.iter().any(|row| {
                rect_top(*image) > rect_top(*row)
                    && (rect_right(*image) - rect_right(*row)).abs() <= tolerance
            })
        }),
        "missing image below table top and flush with table right edge on page {page_index}; images={images:?}; rows={rows:?}"
    );
}

fn table_row_fragment_bounds(document: &LayoutDocument<'_>, page_index: usize) -> Vec<Rect> {
    document
        .frames
        .iter()
        .filter(|frame| frame.page_index == page_index)
        .flat_map(|frame| &frame.fragments)
        .filter_map(|fragment| {
            matches!(fragment.kind, FrameFragmentKind::TableRow)
                .then_some(fragment.bounds)
                .flatten()
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
