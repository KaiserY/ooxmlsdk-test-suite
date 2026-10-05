//! Diagnostic painted-glyph identity, independent of PDF ToUnicode/ActualText.
//!
//! MuPDF resolves nested content streams and font transforms before emitting
//! its XML trace. We use its *painted GID*, then compare unhinted outlines from
//! the PDF's embedded TrueType font. Unknown fonts or glyphs are errors: this
//! module must never infer a glyph from a Unicode character or its PDFium box.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lopdf::{Document, Object};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
    raw::TableProvider,
};

#[derive(Clone, Debug, Serialize)]
pub struct PaintedGlyph {
    pub font: String,
    pub glyph_id: u16,
    pub unicode_hint: Option<String>,
    pub outline_sha256: String,
    pub origin: [f64; 2],
    /// Linear part of the painted text matrix, in PDF user-space points.
    pub paint_linear: [f64; 4],
    pub paint_kind: String,
    pub paint_rgb: Option<[f64; 3]>,
    pub paint_alpha: f64,
    pub stroke: Option<TextStroke>,
}

/// Stroke parameters in page points. Non-conformal transforms and hairlines
/// remain unsupported: their coverage needs more than a scalar stroke width.
#[derive(Clone, Debug, Serialize)]
pub struct TextStroke {
    pub width_pt: f64,
    pub cap: String,
    pub join: String,
    pub miter: f64,
    pub phase_pt: f64,
    pub dashes_pt: Vec<f64>,
}

impl TextStroke {
    fn equivalent(&self, other: &Self) -> bool {
        // Native text-stroke widths serialize to five decimal places. This
        // admits numeric serialization noise, not a visible thickness change.
        let close = |a: f64, b: f64| (a - b).abs() <= 0.00001;
        self.cap == other.cap
            && self.join == other.join
            && close(self.width_pt, other.width_pt)
            && close(self.miter, other.miter)
            && close(self.phase_pt, other.phase_pt)
            && self.dashes_pt.len() == other.dashes_pt.len()
            && self
                .dashes_pt
                .iter()
                .zip(&other.dashes_pt)
                .all(|(a, b)| close(*a, *b))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PaintedGlyphPage {
    pub page_number: usize,
    pub glyphs: Vec<PaintedGlyph>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaintedGlyphComparison {
    pub page_count: usize,
    pub glyph_count: usize,
    pub equal_pages: usize,
    pub differences: Vec<PaintedGlyphPageDifference>,
    pub unsupported_paint_glyphs: usize,
    /// Diagnostic only: neither outline bags nor these measurements grant PASS.
    pub position_diagnostics: Vec<PaintedGlyphPositionDiagnostic>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaintedGlyphPositionDiagnostic {
    pub tolerance_pt: f64,
    pub fully_paired_pages: usize,
    pub candidate_only: usize,
    pub golden_only: usize,
    pub paint_linear_mismatches: usize,
    pub paint_color_mismatches: usize,
    pub paint_state_mismatches: usize,
    pub trace_unicode_mismatches: usize,
    pub page_differences: Vec<PaintedGlyphPositionPageDifference>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaintedGlyphPositionPageDifference {
    pub page_number: usize,
    pub candidate_glyph_count: usize,
    pub golden_glyph_count: usize,
    pub candidate_only: usize,
    pub golden_only: usize,
    pub paint_linear_mismatches: usize,
    pub paint_color_mismatches: usize,
    pub paint_state_mismatches: usize,
    pub trace_unicode_mismatches: usize,
    pub candidate_examples: Vec<PaintedGlyph>,
    pub golden_examples: Vec<PaintedGlyph>,
    pub paint_linear_examples: Vec<PaintedGlyphPairExample>,
    pub paint_color_examples: Vec<PaintedGlyphPairExample>,
    pub trace_unicode_examples: Vec<PaintedGlyphPairExample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaintedGlyphPairExample {
    pub candidate: PaintedGlyph,
    pub golden: PaintedGlyph,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaintedGlyphPageDifference {
    pub page_number: usize,
    pub candidate_glyphs: usize,
    pub golden_glyphs: usize,
    pub candidate_only: usize,
    pub golden_only: usize,
}

pub fn compare_painted_glyphs(
    candidate: &[u8],
    golden: &[u8],
) -> Result<PaintedGlyphComparison, String> {
    let candidate = painted_glyph_pages(candidate)?;
    let golden = painted_glyph_pages(golden)?;
    compare_painted_glyph_pages(&candidate, &golden)
}

/// Stricter than an outline bag: every visible glyph must retain its shape,
/// paint, scale and nearby origin. Small interior justification drift is
/// bounded both per page and over the document, so moving a line cannot pass.
pub fn glyph_backed_text_fidelity(report: &PaintedGlyphComparison) -> Result<(), String> {
    if report.page_count == 0
        || report.glyph_count == 0
        || report.equal_pages != report.page_count
        || report.unsupported_paint_glyphs != 0
    {
        return Err("painted outlines or paint modes are incomplete".into());
    }
    let at = |tolerance: f64| {
        report
            .position_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.tolerance_pt == tolerance)
            .ok_or_else(|| format!("missing {tolerance}pt glyph position diagnostic"))
    };
    let one = at(1.0)?;
    let two = at(2.0)?;
    let four = at(4.0)?;
    if one.candidate_only.saturating_mul(50) > report.glyph_count
        || one.golden_only.saturating_mul(50) > report.glyph_count
        || two.candidate_only.saturating_mul(1000) > report.glyph_count
        || two.golden_only.saturating_mul(1000) > report.glyph_count
        || four.candidate_only != 0
        || four.golden_only != 0
        || four.paint_linear_mismatches != 0
        || four.paint_color_mismatches != 0
        || four.paint_state_mismatches != 0
    {
        return Err(format!(
            "painted glyph position or paint differs: glyphs={}, >1pt={}, >2pt={}, >4pt={}, paint_linear={}, paint_color={}, paint_state={}",
            report.glyph_count,
            one.candidate_only,
            two.candidate_only,
            four.candidate_only,
            four.paint_linear_mismatches,
            four.paint_color_mismatches,
            four.paint_state_mismatches
        ));
    }
    if let Some(page) = two.page_differences.iter().find(|page| {
        page.candidate_only.saturating_mul(50) > page.candidate_glyph_count
            || page.golden_only.saturating_mul(50) > page.golden_glyph_count
    }) {
        return Err(format!(
            "page {} has too many glyphs displaced by more than 2pt: candidate={}/{}, golden={}/{}",
            page.page_number,
            page.candidate_only,
            page.candidate_glyph_count,
            page.golden_only,
            page.golden_glyph_count
        ));
    }
    Ok(())
}

fn compare_painted_glyph_pages(
    candidate: &[PaintedGlyphPage],
    golden: &[PaintedGlyphPage],
) -> Result<PaintedGlyphComparison, String> {
    if candidate.len() != golden.len() {
        return Err(format!(
            "painted-glyph page count mismatch: candidate={} golden={}",
            candidate.len(),
            golden.len()
        ));
    }
    let mut equal_pages = 0;
    let mut differences = Vec::new();
    for (candidate, golden) in candidate.iter().zip(golden) {
        if candidate.page_number != golden.page_number {
            return Err(format!(
                "painted-glyph page number mismatch: candidate={} golden={}",
                candidate.page_number, golden.page_number
            ));
        }
        let candidate_bag = outline_bag(candidate);
        let golden_bag = outline_bag(golden);
        if candidate_bag == golden_bag {
            equal_pages += 1;
        } else {
            let candidate_only = candidate_bag
                .iter()
                .map(|(outline, count)| {
                    count.saturating_sub(*golden_bag.get(outline).unwrap_or(&0))
                })
                .sum();
            let golden_only = golden_bag
                .iter()
                .map(|(outline, count)| {
                    count.saturating_sub(*candidate_bag.get(outline).unwrap_or(&0))
                })
                .sum();
            differences.push(PaintedGlyphPageDifference {
                page_number: candidate.page_number,
                candidate_glyphs: candidate.glyphs.len(),
                golden_glyphs: golden.glyphs.len(),
                candidate_only,
                golden_only,
            });
        }
    }
    let position_diagnostics = [0.35, 1.0, 2.0, 3.0, 4.0, 5.0]
        .into_iter()
        .map(|tolerance_pt| compare_origins(candidate, golden, tolerance_pt))
        .collect();
    Ok(PaintedGlyphComparison {
        page_count: candidate.len(),
        glyph_count: candidate.iter().map(|page| page.glyphs.len()).sum(),
        equal_pages,
        differences,
        unsupported_paint_glyphs: candidate
            .iter()
            .chain(golden)
            .flat_map(|page| &page.glyphs)
            .filter(|glyph| {
                glyph.paint_rgb.is_none()
                    || !glyph.paint_alpha.is_finite()
                    || !(0.0..=1.0).contains(&glyph.paint_alpha)
                    || match glyph.paint_kind.as_str() {
                        "fill_text" => false,
                        "stroke_text" => glyph.stroke.is_none(),
                        _ => true,
                    }
            })
            .count(),
        position_diagnostics,
    })
}

fn compare_origins(
    candidate: &[PaintedGlyphPage],
    golden: &[PaintedGlyphPage],
    tolerance_pt: f64,
) -> PaintedGlyphPositionDiagnostic {
    let mut report = PaintedGlyphPositionDiagnostic {
        tolerance_pt,
        fully_paired_pages: 0,
        candidate_only: 0,
        golden_only: 0,
        paint_linear_mismatches: 0,
        paint_color_mismatches: 0,
        paint_state_mismatches: 0,
        trace_unicode_mismatches: 0,
        page_differences: Vec::new(),
    };
    for (candidate, golden) in candidate.iter().zip(golden) {
        let mut candidate_groups: BTreeMap<(&str, &str), Vec<&PaintedGlyph>> = BTreeMap::new();
        let mut golden_groups: BTreeMap<(&str, &str), Vec<&PaintedGlyph>> = BTreeMap::new();
        for glyph in &candidate.glyphs {
            candidate_groups
                .entry((&glyph.outline_sha256, &glyph.paint_kind))
                .or_default()
                .push(glyph);
        }
        for glyph in &golden.glyphs {
            golden_groups
                .entry((&glyph.outline_sha256, &glyph.paint_kind))
                .or_default()
                .push(glyph);
        }
        let outlines = candidate_groups
            .keys()
            .chain(golden_groups.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        let mut page = PaintedGlyphPositionPageDifference {
            page_number: candidate.page_number,
            candidate_glyph_count: candidate.glyphs.len(),
            golden_glyph_count: golden.glyphs.len(),
            candidate_only: 0,
            golden_only: 0,
            paint_linear_mismatches: 0,
            paint_color_mismatches: 0,
            paint_state_mismatches: 0,
            trace_unicode_mismatches: 0,
            candidate_examples: Vec::new(),
            golden_examples: Vec::new(),
            paint_linear_examples: Vec::new(),
            paint_color_examples: Vec::new(),
            trace_unicode_examples: Vec::new(),
        };
        for outline in outlines {
            let candidate_glyphs = candidate_groups
                .get(&outline)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let golden_glyphs = golden_groups
                .get(&outline)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let pairs = maximum_origin_matching(golden_glyphs, candidate_glyphs, tolerance_pt);
            let mut paired_candidate = vec![false; candidate_glyphs.len()];
            for (index, paired) in pairs.iter().enumerate() {
                if let Some(candidate_index) = paired {
                    paired_candidate[*candidate_index] = true;
                    let reference = golden_glyphs[index];
                    let actual = candidate_glyphs[*candidate_index];
                    let stroke_equal = match (&reference.stroke, &actual.stroke) {
                        (None, None) => true,
                        (Some(a), Some(b)) => a.equivalent(b),
                        _ => false,
                    };
                    if !stroke_equal
                        || (reference.paint_alpha - actual.paint_alpha).abs() > 0.000001
                    {
                        page.paint_state_mismatches += 1;
                    }
                    if reference
                        .paint_linear
                        .iter()
                        .zip(actual.paint_linear)
                        .any(|(a, b)| (a - b).abs() > 0.25)
                    {
                        page.paint_linear_mismatches += 1;
                        if page.paint_linear_examples.len() < 3 {
                            page.paint_linear_examples.push(PaintedGlyphPairExample {
                                candidate: actual.clone(),
                                golden: reference.clone(),
                            });
                        }
                    }
                    if reference
                        .paint_rgb
                        .zip(actual.paint_rgb)
                        .is_none_or(|(a, b)| a.iter().zip(b).any(|(a, b)| (a - b).abs() > 0.01))
                    {
                        page.paint_color_mismatches += 1;
                        if page.paint_color_examples.len() < 3 {
                            page.paint_color_examples.push(PaintedGlyphPairExample {
                                candidate: actual.clone(),
                                golden: reference.clone(),
                            });
                        }
                    }
                    if reference.unicode_hint != actual.unicode_hint {
                        page.trace_unicode_mismatches += 1;
                        if page.trace_unicode_examples.len() < 3 {
                            page.trace_unicode_examples.push(PaintedGlyphPairExample {
                                candidate: actual.clone(),
                                golden: reference.clone(),
                            });
                        }
                    }
                } else {
                    page.golden_only += 1;
                    if page.golden_examples.len() < 3 {
                        page.golden_examples.push(golden_glyphs[index].clone());
                    }
                }
            }
            for (index, glyph) in candidate_glyphs.iter().enumerate() {
                if !paired_candidate[index] {
                    page.candidate_only += 1;
                    if page.candidate_examples.len() < 3 {
                        page.candidate_examples.push((*glyph).clone());
                    }
                }
            }
        }
        if page.candidate_only == 0 && page.golden_only == 0 {
            report.fully_paired_pages += 1;
        }
        report.candidate_only += page.candidate_only;
        report.golden_only += page.golden_only;
        report.paint_linear_mismatches += page.paint_linear_mismatches;
        report.paint_color_mismatches += page.paint_color_mismatches;
        report.paint_state_mismatches += page.paint_state_mismatches;
        report.trace_unicode_mismatches += page.trace_unicode_mismatches;
        if page.candidate_only != 0
            || page.golden_only != 0
            || page.paint_linear_mismatches != 0
            || page.paint_color_mismatches != 0
            || page.paint_state_mismatches != 0
            || page.trace_unicode_mismatches != 0
        {
            report.page_differences.push(page);
        }
    }
    report
}

/// Maximum-cardinality matching: repeated identical outlines can have two
/// nearby candidate origins, where a nearest-first greedy choice strands one.
fn maximum_origin_matching(
    golden: &[&PaintedGlyph],
    candidate: &[&PaintedGlyph],
    tolerance_pt: f64,
) -> Vec<Option<usize>> {
    let tolerance_squared = tolerance_pt * tolerance_pt;
    let edges = golden
        .iter()
        .map(|reference| {
            let mut nearby = candidate
                .iter()
                .enumerate()
                .filter_map(|(index, actual)| {
                    let dx = reference.origin[0] - actual.origin[0];
                    let dy = reference.origin[1] - actual.origin[1];
                    let distance_squared = dx * dx + dy * dy;
                    (distance_squared <= tolerance_squared).then_some((distance_squared, index))
                })
                .collect::<Vec<_>>();
            nearby.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            nearby.into_iter().map(|(_, index)| index).collect()
        })
        .collect::<Vec<Vec<usize>>>();
    let mut assigned = vec![None; candidate.len()];
    for index in 0..golden.len() {
        let mut visited = vec![false; candidate.len()];
        augment_origin_matching(index, &edges, &mut assigned, &mut visited);
    }
    let mut pairs = vec![None; golden.len()];
    for (candidate_index, golden_index) in assigned.into_iter().enumerate() {
        if let Some(golden_index) = golden_index {
            pairs[golden_index] = Some(candidate_index);
        }
    }
    pairs
}

fn augment_origin_matching(
    golden_index: usize,
    edges: &[Vec<usize>],
    assigned: &mut [Option<usize>],
    visited: &mut [bool],
) -> bool {
    for &candidate_index in &edges[golden_index] {
        if visited[candidate_index] {
            continue;
        }
        visited[candidate_index] = true;
        let previous = assigned[candidate_index];
        if previous.is_none_or(|other| augment_origin_matching(other, edges, assigned, visited)) {
            assigned[candidate_index] = Some(golden_index);
            return true;
        }
    }
    false
}

fn outline_bag(page: &PaintedGlyphPage) -> BTreeMap<(&str, &str), usize> {
    let mut bag = BTreeMap::new();
    for glyph in &page.glyphs {
        *bag.entry((glyph.outline_sha256.as_str(), glyph.paint_kind.as_str()))
            .or_default() += 1;
    }
    bag
}

pub fn painted_glyph_pages(pdf: &[u8]) -> Result<Vec<PaintedGlyphPage>, String> {
    let document = Document::load_mem(pdf).map_err(|error| format!("invalid PDF: {error}"))?;
    let mut fonts = embedded_truetype_fonts(&document)?;
    let trace = mutool_trace(pdf)?;
    let mut pages = parse_trace(&trace, &mut fonts)?;
    if pages
        .iter()
        .flat_map(|page| &page.glyphs)
        .any(|glyph| glyph.paint_kind == "stroke_text")
    {
        attach_text_strokes(pdf, &mut pages, &mut fonts)?;
    }
    Ok(pages)
}

struct EmbeddedFont {
    bytes: Vec<u8>,
    outlines: HashMap<u16, Option<String>>,
    glyph_names: Option<HashMap<String, Option<u16>>>,
}

impl EmbeddedFont {
    fn glyph_id_from_name(&mut self, name: &str) -> Result<u16, String> {
        if self.glyph_names.is_none() {
            let face = FontRef::from_index(&self.bytes, 0)
                .map_err(|error| format!("invalid embedded TrueType font: {error:?}"))?;
            let mut names = HashMap::new();
            for (glyph_id, glyph_name) in face.glyph_names().iter() {
                let glyph_id = u16::try_from(glyph_id.to_u32()).map_err(|_| {
                    format!("embedded font glyph {glyph_id:?} exceeds PDF GID range")
                })?;
                names
                    .entry(glyph_name.as_str().to_owned())
                    .and_modify(|value| *value = None)
                    .or_insert(Some(glyph_id));
            }
            self.glyph_names = Some(names);
        }
        match self.glyph_names.as_ref().and_then(|names| names.get(name)) {
            Some(Some(glyph_id)) => Ok(*glyph_id),
            Some(None) => Err(format!("painted glyph name {name:?} is ambiguous")),
            None => Err(format!(
                "painted glyph name {name:?} is absent from embedded font"
            )),
        }
    }

    fn outline(&mut self, glyph_id: u16) -> Result<Option<String>, String> {
        if let Some(outline) = self.outlines.get(&glyph_id) {
            return Ok(outline.clone());
        }
        let face = FontRef::from_index(&self.bytes, 0)
            .map_err(|error| format!("invalid embedded TrueType font: {error:?}"))?;
        let glyph_count = face
            .maxp()
            .map_err(|error| format!("embedded TrueType font has no maxp: {error:?}"))?
            .num_glyphs();
        if glyph_id >= glyph_count {
            return Err(format!(
                "painted GID {glyph_id} exceeds embedded font glyph count {}",
                glyph_count
            ));
        }
        let upem = face
            .head()
            .map_err(|error| format!("embedded TrueType font has no head: {error:?}"))?
            .units_per_em();
        let mut pen = OutlineDigest::new(upem);
        let outline = face
            .outline_glyphs()
            .get(GlyphId::new(u32::from(glyph_id)))
            .ok_or_else(|| format!("painted GID {glyph_id} has no TrueType outline"))?;
        outline
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .map_err(|error| format!("could not draw painted GID {glyph_id}: {error:?}"))?;
        let outline = pen.has_ink.then(|| format!("{:x}", pen.hasher.finalize()));
        self.outlines.insert(glyph_id, outline.clone());
        Ok(outline)
    }
}

struct OutlineDigest {
    hasher: Sha256,
    upem: f64,
    has_ink: bool,
}

impl OutlineDigest {
    fn new(upem: u16) -> Self {
        Self {
            hasher: Sha256::new(),
            upem: f64::from(upem.max(1)),
            has_ink: false,
        }
    }

    fn point(&mut self, x: f32, y: f32) {
        for coordinate in [x, y] {
            let normalized = (f64::from(coordinate) / self.upem * 1_000_000_000.0).round() as i64;
            self.hasher.update(normalized.to_be_bytes());
        }
    }
}

impl OutlinePen for OutlineDigest {
    fn move_to(&mut self, x: f32, y: f32) {
        self.hasher.update(b"M");
        self.point(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.hasher.update(b"L");
        self.point(x, y);
        self.has_ink = true;
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.hasher.update(b"Q");
        self.point(x1, y1);
        self.point(x, y);
        self.has_ink = true;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.hasher.update(b"C");
        self.point(x1, y1);
        self.point(x2, y2);
        self.point(x, y);
        self.has_ink = true;
    }
    fn close(&mut self) {
        self.hasher.update(b"Z");
    }
}

fn embedded_truetype_fonts(document: &Document) -> Result<HashMap<String, EmbeddedFont>, String> {
    let mut fonts: HashMap<String, EmbeddedFont> = HashMap::new();
    for object in document.objects.values() {
        let Ok(font) = object.as_dict() else {
            continue;
        };
        if !font
            .get(b"Type")
            .ok()
            .is_some_and(|value| value.as_name().ok() == Some(b"Font"))
        {
            continue;
        }
        let Some(name) = font
            .get(b"BaseFont")
            .ok()
            .and_then(|value| value.as_name().ok())
        else {
            continue;
        };
        let Some(descendant) = font
            .get(b"DescendantFonts")
            .ok()
            .and_then(|value| resolve(document, value).ok())
            .and_then(|value| value.as_array().ok())
            .and_then(|values| values.first())
            .and_then(|value| resolve(document, value).ok())
            .or(Some(object))
        else {
            continue;
        };
        let Ok(face) = descendant.as_dict() else {
            continue;
        };
        let Some(stream) = face
            .get(b"FontDescriptor")
            .ok()
            .and_then(|value| resolve(document, value).ok())
            .and_then(|value| value.as_dict().ok())
            .and_then(|value| value.get(b"FontFile2").ok())
            .and_then(|value| resolve(document, value).ok())
            .and_then(|value| value.as_stream().ok())
        else {
            continue;
        };
        let bytes = stream
            .decompressed_content()
            .map_err(|error| format!("could not decompress FontFile2: {error}"))?;
        let name = String::from_utf8(name.to_vec())
            .map_err(|error| format!("non-UTF8 PDF BaseFont name: {error}"))?;
        if !name.is_ascii() {
            return Err(format!("non-ASCII PDF BaseFont name {name:?}"));
        }
        if let Some(existing) = fonts.get(&name) {
            if existing.bytes != bytes {
                return Err(format!("ambiguous embedded font name {name:?}"));
            }
            continue;
        }
        fonts.insert(
            name,
            EmbeddedFont {
                bytes,
                outlines: HashMap::new(),
                glyph_names: None,
            },
        );
    }
    // MuPDF limits trace font names to 31 bytes. Reject ambiguous prefixes.
    let aliases = fonts
        .keys()
        .filter(|name| name.len() > 31)
        .map(|name| (name[..31].to_string(), name.clone()))
        .collect::<Vec<_>>();
    for (alias, full) in aliases {
        if let Some(existing) = fonts.get(&alias) {
            if existing.bytes != fonts[&full].bytes {
                return Err(format!("ambiguous MuPDF font alias {alias:?}"));
            }
            continue;
        }
        let bytes = fonts[&full].bytes.clone();
        fonts.insert(
            alias,
            EmbeddedFont {
                bytes,
                outlines: HashMap::new(),
                glyph_names: None,
            },
        );
    }
    Ok(fonts)
}

fn resolve<'a>(document: &'a Document, mut object: &'a Object) -> Result<&'a Object, String> {
    for _ in 0..64 {
        let Object::Reference(id) = object else {
            return Ok(object);
        };
        object = document
            .get_object(*id)
            .map_err(|error| error.to_string())?;
    }
    Err("PDF reference chain exceeds 64 levels".into())
}

fn mutool_trace(pdf: &[u8]) -> Result<Vec<u8>, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ooxmlsdk-painted-glyphs-{}-{nanos}.pdf",
        std::process::id()
    ));
    std::fs::write(&path, pdf).map_err(|error| error.to_string())?;
    let output = Command::new("mutool")
        .args(["draw", "-q", "-F", "trace", "-o", "-"])
        .arg(&path)
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.map_err(|error| format!("mutool trace could not start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "mutool trace failed: status={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

#[derive(Deserialize)]
struct StrokeGlyph {
    page_number: usize,
    font: String,
    glyph_id: u16,
    origin: [f64; 2],
    stroke: DeviceTextStroke,
}

#[derive(Deserialize)]
struct DeviceTextStroke {
    width: f64,
    cap: String,
    join: String,
    miter: f64,
    phase: f64,
    dashes: Vec<f64>,
    transform: [f64; 4],
    alpha: f64,
}

impl DeviceTextStroke {
    fn in_page_points(&self) -> Option<TextStroke> {
        let [a, b, c, d] = self.transform;
        let x2 = a * a + b * b;
        let y2 = c * c + d * d;
        if !self.transform.iter().all(|value| value.is_finite())
            || x2 <= 0.0
            || (x2 - y2).abs() > x2 * 0.000001
            || (a * c + b * d).abs() > x2 * 0.000001
            || !self.width.is_finite()
            || self.width <= 0.0
            || !self.miter.is_finite()
            || self.miter < 1.0
            || !self.phase.is_finite()
            || self
                .dashes
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            || !matches!(self.cap.as_str(), "Butt" | "Round" | "Square")
            || !matches!(self.join.as_str(), "Miter" | "Round" | "Bevel")
        {
            return None;
        }
        let scale = x2.sqrt();
        Some(TextStroke {
            width_pt: self.width * scale,
            cap: self.cap.clone(),
            join: self.join.clone(),
            miter: self.miter,
            phase_pt: self.phase * scale,
            dashes_pt: self.dashes.iter().map(|value| value * scale).collect(),
        })
    }
}

fn attach_text_strokes(
    pdf: &[u8],
    pages: &mut [PaintedGlyphPage],
    fonts: &mut HashMap<String, EmbeddedFont>,
) -> Result<(), String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "ooxmlsdk-text-strokes-{}-{nanos}",
        std::process::id()
    ));
    let path = base.with_extension("pdf");
    let script = base.with_extension("js");
    let output = (|| {
        std::fs::write(&path, pdf).map_err(|error| error.to_string())?;
        std::fs::write(&script, include_str!("pdf_text_strokes.js"))
            .map_err(|error| error.to_string())?;
        Command::new("mutool")
            .arg("run")
            .arg(&script)
            .arg(&path)
            .output()
            .map_err(|error| format!("MuPDF text-stroke device could not start: {error}"))
    })();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&script);
    let output = output?;
    if !output.status.success() {
        return Err(format!(
            "MuPDF text-stroke device failed: {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut visible = Vec::new();
    for line in output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let glyph: StrokeGlyph = serde_json::from_slice(line)
            .map_err(|error| format!("invalid MuPDF stroke record: {error}"))?;
        let font = fonts
            .get_mut(&glyph.font)
            .ok_or_else(|| format!("stroke font {:?} is not embedded", glyph.font))?;
        if font.outline(glyph.glyph_id)?.is_some() {
            visible.push(glyph);
        }
    }
    let mut strokes = visible.into_iter();
    for page in pages {
        for glyph in page
            .glyphs
            .iter_mut()
            .filter(|glyph| glyph.paint_kind == "stroke_text")
        {
            let state = strokes
                .next()
                .ok_or("MuPDF stroke device omitted a painted glyph")?;
            // The XML device truncates font names to 31 ASCII bytes. Both
            // collectors must identify exactly the same ordered paint event.
            let alias = &state.font[..state.font.len().min(31)];
            if state.page_number != page.page_number
                || alias != glyph.font
                || state.glyph_id != glyph.glyph_id
                || state
                    .origin
                    .iter()
                    .zip(glyph.origin)
                    .any(|(a, b)| !a.is_finite() || (a - b).abs() > 0.001)
                || (state.stroke.alpha - glyph.paint_alpha).abs() > 0.000001
            {
                return Err("MuPDF XML and stroke-device glyph identities disagree".into());
            }
            glyph.stroke = state.stroke.in_page_points();
        }
    }
    if strokes.next().is_some() {
        return Err("MuPDF stroke device contains extra painted glyphs".into());
    }
    Ok(())
}

fn attribute(element: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, String> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| error.to_string())?;
        if attribute.key.as_ref() == key {
            return String::from_utf8(attribute.value.into_owned())
                .map(Some)
                .map_err(|error| error.to_string());
        }
    }
    Ok(None)
}

fn required_attribute(element: &BytesStart<'_>, key: &[u8]) -> Result<String, String> {
    attribute(element, key)?.ok_or_else(|| {
        format!(
            "MuPDF {:?} element lacks {:?}",
            String::from_utf8_lossy(element.name().as_ref()),
            String::from_utf8_lossy(key)
        )
    })
}

fn parse_transform(value: &str) -> Result<[f64; 6], String> {
    let values = value
        .split_ascii_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("invalid MuPDF text transform: {error}"))?;
    values
        .try_into()
        .map_err(|_| format!("invalid MuPDF text transform {value:?}"))
}

fn parse_text_linear(value: &str) -> Result<[f64; 4], String> {
    let values = value
        .split_ascii_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("invalid MuPDF text matrix: {error}"))?;
    values
        .try_into()
        .map_err(|_| format!("invalid MuPDF text matrix {value:?}"))
}

fn parse_paint_rgb(element: &BytesStart<'_>) -> Result<Option<[f64; 3]>, String> {
    let Some(colorspace) = attribute(element, b"colorspace")? else {
        return Ok(None);
    };
    let Some(color) = attribute(element, b"color")? else {
        return Ok(None);
    };
    let channels = color
        .split_ascii_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("invalid MuPDF paint color: {error}"))?;
    match (colorspace.as_str(), channels.as_slice()) {
        ("DeviceGray", [gray]) if gray.is_finite() => Ok(Some([*gray; 3])),
        ("DeviceRGB", [red, green, blue])
            if [red, green, blue].iter().all(|channel| channel.is_finite()) =>
        {
            Ok(Some([*red, *green, *blue]))
        }
        _ => Ok(None),
    }
}

fn parse_trace(
    xml: &[u8],
    fonts: &mut HashMap<String, EmbeddedFont>,
) -> Result<Vec<PaintedGlyphPage>, String> {
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut pages = Vec::<PaintedGlyphPage>::new();
    let mut paint = false;
    let mut paint_kind = String::new();
    let mut paint_rgb = None;
    let mut paint_alpha = 1.0;
    let mut transform = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut text_transform = [1.0, 0.0, 0.0, 1.0];
    let mut font = None::<String>;
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| error.to_string())?
        {
            Event::Start(element) if element.name().as_ref() == b"page" => {
                let number = required_attribute(&element, b"number")?
                    .parse::<usize>()
                    .map_err(|error| format!("invalid MuPDF page number: {error}"))?;
                pages.push(PaintedGlyphPage {
                    page_number: number,
                    glyphs: Vec::new(),
                });
            }
            Event::Start(element) if is_painted_text(element.name().as_ref()) => {
                paint = true;
                paint_kind = String::from_utf8(element.name().as_ref().to_vec())
                    .map_err(|error| error.to_string())?;
                paint_rgb = parse_paint_rgb(&element)?;
                paint_alpha = attribute(&element, b"alpha")?
                    .map(|value| value.parse::<f64>())
                    .transpose()
                    .map_err(|error| format!("invalid MuPDF paint alpha: {error}"))?
                    .unwrap_or(1.0);
                transform = parse_transform(&required_attribute(&element, b"transform")?)?;
            }
            Event::Start(element) if element.name().as_ref() == b"span" && paint => {
                font = Some(required_attribute(&element, b"font")?);
                text_transform = parse_text_linear(&required_attribute(&element, b"trm")?)?;
            }
            Event::Empty(element) if element.name().as_ref() == b"g" && paint => {
                let font_name = font
                    .as_ref()
                    .ok_or("painted MuPDF glyph has no font span")?;
                let Some(raw_glyph_id) = attribute(&element, b"glyph")? else {
                    // MuPDF emits a second Unicode scalar for some clusters
                    // without another painted glyph (for example Arabic lam-alef).
                    buffer.clear();
                    continue;
                };
                if raw_glyph_id == "-1" {
                    // MuPDF repeats a Unicode continuation with GID -1 for a
                    // multi-scalar cluster; it is not another painted glyph.
                    buffer.clear();
                    continue;
                }
                let embedded = fonts.get_mut(font_name).ok_or_else(|| {
                    format!("painted font {font_name:?} has no embedded TrueType")
                })?;
                let glyph_id = match raw_glyph_id.parse::<i32>() {
                    Ok(number) => u16::try_from(number)
                        .map_err(|_| format!("MuPDF glyph ID {number} is out of range"))?,
                    Err(_) => embedded.glyph_id_from_name(&raw_glyph_id)?,
                };
                let Some(outline_sha256) = embedded.outline(glyph_id)? else {
                    buffer.clear();
                    continue;
                };
                let x = required_attribute(&element, b"x")?
                    .parse::<f64>()
                    .map_err(|error| format!("invalid glyph x: {error}"))?;
                let y = required_attribute(&element, b"y")?
                    .parse::<f64>()
                    .map_err(|error| format!("invalid glyph y: {error}"))?;
                let [a, b, c, d, e, f] = transform;
                let [ta, tb, tc, td] = text_transform;
                pages
                    .last_mut()
                    .ok_or("painted glyph appears before first page")?
                    .glyphs
                    .push(PaintedGlyph {
                        font: font_name.clone(),
                        glyph_id,
                        unicode_hint: attribute(&element, b"unicode")?,
                        outline_sha256,
                        origin: [a * x + c * y + e, b * x + d * y + f],
                        paint_linear: [
                            a * ta + c * tb,
                            b * ta + d * tb,
                            a * tc + c * td,
                            b * tc + d * td,
                        ],
                        paint_kind: paint_kind.clone(),
                        paint_rgb,
                        paint_alpha,
                        stroke: None,
                    });
            }
            Event::End(element) if element.name().as_ref() == b"span" => font = None,
            Event::End(element) if is_painted_text(element.name().as_ref()) => paint = false,
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    if pages.is_empty() {
        return Err("MuPDF trace has no pages".into());
    }
    Ok(pages)
}

fn is_painted_text(tag: &[u8]) -> bool {
    matches!(
        tag,
        b"fill_text" | b"stroke_text" | b"fill_stroke_text" | b"clip_stroke_text"
    )
}

pub fn compare_pdf_files(
    candidate: &Path,
    golden: &Path,
) -> Result<PaintedGlyphComparison, String> {
    let candidate = std::fs::read(candidate).map_err(|error| error.to_string())?;
    let golden = std::fs::read(golden).map_err(|error| error.to_string())?;
    compare_painted_glyphs(&candidate, &golden)
}

#[cfg(test)]
mod tests {
    use super::{
        DeviceTextStroke, PaintedGlyph, PaintedGlyphPage, compare_painted_glyph_pages,
        glyph_backed_text_fidelity,
    };

    fn glyph(outline: &str, x: f64, size: f64, unicode: &str) -> PaintedGlyph {
        PaintedGlyph {
            font: "test-font".into(),
            glyph_id: 1,
            unicode_hint: Some(unicode.into()),
            outline_sha256: outline.into(),
            origin: [x, 0.0],
            paint_linear: [size, 0.0, 0.0, -size],
            paint_kind: "fill_text".into(),
            paint_rgb: Some([0.0; 3]),
            paint_alpha: 1.0,
            stroke: None,
        }
    }

    fn page(glyphs: Vec<PaintedGlyph>) -> Vec<PaintedGlyphPage> {
        vec![PaintedGlyphPage {
            page_number: 1,
            glyphs,
        }]
    }

    #[test]
    fn repeated_outlines_use_maximum_pairing_instead_of_greedy_nearest() {
        // Golden 0 can use either candidate; golden 1.5 can only use 0.5.
        // Greedy nearest would consume 0.5 for golden 0 and report a false gap.
        let golden = page(vec![glyph("A", 0.0, 10.0, "a"), glyph("A", 1.5, 10.0, "a")]);
        let candidate = page(vec![
            glyph("A", 0.5, 10.0, "a"),
            glyph("A", -0.8, 10.0, "a"),
        ]);
        let result = compare_painted_glyph_pages(&candidate, &golden).unwrap();
        let tolerance = &result.position_diagnostics[1];
        assert_eq!(tolerance.tolerance_pt, 1.0);
        assert_eq!(tolerance.fully_paired_pages, 1);
        assert_eq!(tolerance.candidate_only, 0);
        assert_eq!(tolerance.golden_only, 0);
        let tolerance = &result.position_diagnostics[2];
        assert_eq!(tolerance.tolerance_pt, 2.0);
        assert_eq!(tolerance.fully_paired_pages, 1);
        assert_eq!(tolerance.candidate_only, 0);
        assert_eq!(tolerance.golden_only, 0);
    }

    #[test]
    fn equal_outline_inventory_does_not_hide_position_or_size_or_unicode() {
        let golden = page(vec![
            glyph("A", 0.0, 10.0, "a"),
            glyph("B", 10.0, 10.0, "b"),
        ]);
        let candidate = page(vec![
            glyph("A", 0.0, 11.0, "x"),
            glyph("B", 13.0, 10.0, "b"),
        ]);
        let result = compare_painted_glyph_pages(&candidate, &golden).unwrap();
        assert_eq!(result.equal_pages, 1);
        let tolerance = &result.position_diagnostics[1];
        assert_eq!(tolerance.golden_only, 1);
        assert_eq!(tolerance.candidate_only, 1);
        assert_eq!(tolerance.paint_linear_mismatches, 1);
        assert_eq!(tolerance.trace_unicode_mismatches, 1);
        assert_eq!(
            tolerance.page_differences[0].golden_examples[0].origin,
            [10.0, 0.0]
        );
    }

    #[test]
    fn changed_visible_outline_is_reported_separately() {
        let golden = page(vec![glyph("A", 0.0, 10.0, "a")]);
        let candidate = page(vec![glyph("B", 0.0, 10.0, "a")]);
        let result = compare_painted_glyph_pages(&candidate, &golden).unwrap();
        assert_eq!(result.equal_pages, 0);
        assert_eq!(result.differences[0].golden_only, 1);
        assert_eq!(result.differences[0].candidate_only, 1);
    }

    #[test]
    fn source_unicode_alias_is_visual_but_displaced_or_recolored_glyph_is_not() {
        let golden = page(vec![glyph("A", 0.0, 10.0, "ي")]);
        let alias = page(vec![glyph("A", 0.2, 10.0, "ی")]);
        let report = compare_painted_glyph_pages(&alias, &golden).unwrap();
        assert_eq!(report.position_diagnostics[0].trace_unicode_mismatches, 1);
        glyph_backed_text_fidelity(&report).unwrap();

        let displaced = page(vec![glyph("A", 4.1, 10.0, "ي")]);
        let report = compare_painted_glyph_pages(&displaced, &golden).unwrap();
        assert!(glyph_backed_text_fidelity(&report).is_err());

        let mut red = glyph("A", 0.0, 10.0, "ي");
        red.paint_rgb = Some([1.0, 0.0, 0.0]);
        let report = compare_painted_glyph_pages(&page(vec![red]), &golden).unwrap();
        assert_eq!(report.position_diagnostics[4].paint_color_mismatches, 1);
        assert!(glyph_backed_text_fidelity(&report).is_err());
    }

    #[test]
    fn stroke_proof_preserves_width_caps_joins_dashes_opacity_and_paint_kind() {
        let device = DeviceTextStroke {
            width: 0.27,
            cap: "Butt".into(),
            join: "Miter".into(),
            miter: 10.0,
            phase: 0.0,
            dashes: Vec::new(),
            transform: [1.0, 0.0, 0.0, -1.0],
            alpha: 1.0,
        };
        let mut outline = glyph("A", 0.0, 9.48, "a");
        outline.paint_kind = "stroke_text".into();
        outline.stroke = device.in_page_points();
        // Fill and stroke share an outline and origin. They must not pair
        // with one another when their PDF emission order differs.
        let fill = glyph("A", 0.0, 9.48, "a");
        let golden = page(vec![fill.clone(), outline.clone()]);
        let candidate = page(vec![outline.clone(), fill.clone()]);
        glyph_backed_text_fidelity(&compare_painted_glyph_pages(&candidate, &golden).unwrap())
            .unwrap();

        for property in 0..9 {
            let mut changed = outline.clone();
            let stroke = changed.stroke.as_mut().unwrap();
            match property {
                0 => stroke.width_pt = 0.316,
                1 => stroke.cap = "Round".into(),
                2 => stroke.join = "Bevel".into(),
                3 => stroke.miter = 2.0,
                4 => stroke.phase_pt = 1.0,
                5 => stroke.dashes_pt = vec![1.0, 1.0],
                6 => changed.paint_alpha = 0.5,
                7 => changed.paint_kind = "fill_text".into(),
                8 => changed.stroke = None,
                _ => unreachable!(),
            }
            let candidate = page(vec![fill.clone(), changed]);
            let report = compare_painted_glyph_pages(&candidate, &golden).unwrap();
            assert!(
                glyph_backed_text_fidelity(&report).is_err(),
                "property {property}"
            );
        }
        let mut unsupported = device;
        unsupported.width = 0.0;
        assert!(unsupported.in_page_points().is_none());
        unsupported.width = 0.27;
        unsupported.transform = [1.0, 0.0, 0.2, 1.0];
        assert!(unsupported.in_page_points().is_none());
    }
}
