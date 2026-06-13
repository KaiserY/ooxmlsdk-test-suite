use ooxmlsdk_formula::{
    AddressFlags, CellAddress, CellRange, QualifiedAddress, QualifiedRange, SheetId,
};

const XLSX_MAX_COLUMN_ZERO_BASED: u32 = 16_383;
const XLSX_MAX_ROW_ZERO_BASED: u32 = 1_048_575;

#[test]
fn parses_quoted_sheet_names_and_absolute_address_flags() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFormulaParseReference.
    // LibreOffice's native grammar uses '.' between sheet and cell; OOXML formulas use '!'.
    let checks = [
        (
            "'90''s Music'!D10",
            "90's Music",
            CellAddress { column: 3, row: 9 },
            AddressFlags::default(),
        ),
        (
            "'90''s and 70''s'!$AB$100",
            "90's and 70's",
            CellAddress {
                column: 27,
                row: 99,
            },
            AddressFlags {
                absolute_column: true,
                absolute_row: true,
                ..AddressFlags::default()
            },
        ),
        (
            "'All Others'!Z$100",
            "All Others",
            CellAddress {
                column: 25,
                row: 99,
            },
            AddressFlags {
                absolute_row: true,
                ..AddressFlags::default()
            },
        ),
        (
            "NoQuote!$C111",
            "NoQuote",
            CellAddress {
                column: 2,
                row: 110,
            },
            AddressFlags {
                absolute_column: true,
                ..AddressFlags::default()
            },
        ),
    ];

    for (formula, sheet_name, cell, flags) in checks {
        let parsed = QualifiedAddress::parse_a1(SheetId(0), formula).unwrap();
        assert_eq!(parsed.sheet, SheetId(0), "{formula}");
        assert_eq!(parsed.sheet_name.unwrap().0, sheet_name, "{formula}");
        assert_eq!(parsed.cell, cell, "{formula}");
        assert_eq!(parsed.flags, flags, "{formula}");
    }
}

#[test]
fn rejects_open_ended_reference_bounds() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFormulaParseReference.
    for value in [
        ":B",
        "B:",
        ":B2",
        "B2:",
        ":2",
        "2:",
        ":2B",
        "2B:",
        "abc_foo:abc_bar",
        "B1:B2~C1",
    ] {
        assert!(CellRange::parse_a1(value).is_err(), "{value}");
    }
}

#[test]
fn parses_whole_column_and_whole_row_ranges() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula2.cxx::testRefR1C1WholeCol
    // and testRefR1C1WholeRow. The LO tests compile R1C1 to A1 strings; this
    // test verifies the resulting A1 ranges accepted by ooxmlsdk-formula.
    let column = QualifiedRange::parse_a1(SheetId(0), "L:L").unwrap();
    assert_eq!(column.range.start, CellAddress { column: 11, row: 0 });
    assert_eq!(
        column.range.end,
        CellAddress {
            column: 11,
            row: XLSX_MAX_ROW_ZERO_BASED
        }
    );
    assert!(column.start_flags.whole_column);
    assert!(column.end_flags.whole_column);

    let row = QualifiedRange::parse_a1(SheetId(0), "5:5").unwrap();
    assert_eq!(row.range.start, CellAddress { column: 0, row: 4 });
    assert_eq!(
        row.range.end,
        CellAddress {
            column: XLSX_MAX_COLUMN_ZERO_BASED,
            row: 4
        }
    );
    assert!(row.start_flags.whole_row);
    assert!(row.end_flags.whole_row);
}

#[test]
fn parses_whole_axis_ranges_from_libreoffice_reference_cases() {
    // Source: LibreOffice sc/qa/unit/ucalc_formula.cxx::testFormulaParseReference.
    let column = QualifiedRange::parse_a1(SheetId(0), "B:B").unwrap();
    assert_eq!(column.range.start, CellAddress { column: 1, row: 0 });
    assert_eq!(
        column.range.end,
        CellAddress {
            column: 1,
            row: XLSX_MAX_ROW_ZERO_BASED
        }
    );
    assert!(column.start_flags.whole_column);
    assert!(column.end_flags.whole_column);

    let row = QualifiedRange::parse_a1(SheetId(0), "2:2").unwrap();
    assert_eq!(row.range.start, CellAddress { column: 0, row: 1 });
    assert_eq!(
        row.range.end,
        CellAddress {
            column: XLSX_MAX_COLUMN_ZERO_BASED,
            row: 1
        }
    );
    assert!(row.start_flags.whole_row);
    assert!(row.end_flags.whole_row);

    let sheet_range = QualifiedRange::parse_a1(SheetId(4), "NoQuote!B:C").unwrap();
    assert_eq!(sheet_range.sheet, SheetId(4));
    assert_eq!(sheet_range.sheet_name.unwrap().0, "NoQuote");
    assert_eq!(sheet_range.range.start, CellAddress { column: 1, row: 0 });
    assert_eq!(
        sheet_range.range.end,
        CellAddress {
            column: 2,
            row: XLSX_MAX_ROW_ZERO_BASED
        }
    );
    assert!(sheet_range.start_flags.whole_column);
    assert!(sheet_range.end_flags.whole_column);
}
