use ooxmlsdk_pdf_test::{libreoffice_fixture, pdf_summary_for_fixture};

#[test]
fn arial_bold_chart_text_exports_at_its_authored_em_size() {
    // These unchanged Office PDFs retain equal X/Y em dimensions for
    // Arial-BoldMT: 11.04pt after Excel's printer-grid rounding. Family and
    // weight do not authorize the old renderer's additional 7% Y stretch.
    for (file, expected) in [
        ("../Apache-POI/test-data/integration/stress024.xlsx", "150"),
        (
            "../Apache-POI/test-data/spreadsheet/57181.xlsm",
            "Aufgliederung der für alle Piloten empfangenen Anrufe",
        ),
    ] {
        let summary = pdf_summary_for_fixture(&libreoffice_fixture(file)).unwrap();
        // Script/font runs may split a title inside a word (for example at
        // the ü in "für"). Keep exact text and contiguous page ownership.
        let title = summary
            .text_objects
            .iter()
            .enumerate()
            .find_map(|(start, first)| {
                if first.text.is_empty()
                    || !expected.starts_with(&first.text)
                    || !first.font_name.contains("Arial")
                    || !first.font_name.contains("Bold")
                    || (first.unscaled_font_size.parse::<f32>().unwrap() - 11.0).abs() >= 0.05
                {
                    return None;
                }
                let mut reconstructed = String::new();
                for (offset, text) in summary.text_objects[start..].iter().enumerate() {
                    if text.page_index != first.page_index {
                        break;
                    }
                    reconstructed.push_str(&text.text);
                    if reconstructed == expected {
                        return Some(&summary.text_objects[start..=start + offset]);
                    }
                    if !expected.starts_with(&reconstructed) {
                        break;
                    }
                }
                None
            })
            .unwrap_or_else(|| panic!("missing exact chart text {expected:?}: {file}"));
        for text in title {
            assert!(
                text.font_name.contains("Arial") && text.font_name.contains("Bold"),
                "{file}: {text:?}"
            );
            assert!(
                (text.unscaled_font_size.parse::<f32>().unwrap() - 11.0).abs() < 0.05,
                "{file}: {text:?}"
            );
            assert_eq!(
                text.scaled_font_size, text.unscaled_font_size,
                "{file}: {text:?}"
            );
        }
    }
}
