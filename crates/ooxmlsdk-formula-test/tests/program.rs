use ooxmlsdk_formula::program::{
    FormulaEditError, FormulaEditOp, FormulaEditRange, FormulaEditStatus, FormulaPrintOptions,
    FormulaProgram, FormulaProgramBuilder, FormulaStructuredReferenceItem,
    FormulaStructuredReferencePart, FormulaStructuredReferenceSpecifier,
};
use ooxmlsdk_formula::source::FormulaSource;
use ooxmlsdk_formula::{CellAddress, CellRange, SheetId};

fn print_source(formula: &str) -> String {
    FormulaProgram::from_source(FormulaSource {
        text: formula,
        context: Default::default(),
    })
    .print_formula(&FormulaPrintOptions::default())
    .expect("formula program should print")
}

fn try_print_source(formula: &str) -> Option<String> {
    FormulaProgram::from_source(FormulaSource {
        text: formula,
        context: Default::default(),
    })
    .print_formula(&FormulaPrintOptions::default())
}

#[test]
fn prints_formula_program_from_source() {
    // Source: LibreOffice ScCompiler token array printing and Apache POI FormulaRenderer
    // both render from the compiled formula program/token representation.
    for (source, expected) in [
        ("=SUM(A1:B2,3)", "SUM(A1:B2,3)"),
        ("A1+B2*C3", "A1+B2*C3"),
        ("(A1+B1)*C1", "(A1+B1)*C1"),
        ("IF(A1,,B1)", "IF(A1,,B1)"),
        ("@SUM(A1:A2)", "@SUM(A1:A2)"),
        ("A1 B1+C1", "A1 B1+C1"),
        ("A1+B1 C1", "A1+B1 C1"),
        ("2^3^4", "2^(3^4)"),
        ("\"a\"\"b\"&C1", "\"a\"\"b\"&C1"),
        ("1E+2+A1", "100+A1"),
        (".25*4", "0.25*4"),
        ("{1,2;3,4}", "{1,2;3,4}"),
        ("Sheet1!A1", "Sheet1!A1"),
        ("'My Sheet'!$A$1:$B$2", "'My Sheet'!$A$1:$B$2"),
        ("[Book.xlsx]'O''Brien'!A1", "[Book.xlsx]'O''Brien'!A1"),
        ("Gender_lookup[]", "Gender_lookup[]"),
        ("Table1[Amount]", "Table1[Amount]"),
        ("Table1[[Amount]:[Total]]", "Table1[[Amount]:[Total]]"),
        (
            "Member_Data[[#This Row],[Gender ]]",
            "Member_Data[[#This Row],[Gender ]]",
        ),
        ("#DIV/0!", "#DIV/0!"),
    ] {
        assert_eq!(print_source(source), expected, "{source}");
    }
}

#[test]
fn parse_print_parse_keeps_formula_program_printable() {
    // Source: LibreOffice and Apache POI both write formulas from their compiled token
    // representation rather than preserving the original string byte-for-byte.
    for source in [
        "SUM(A1:B2,3)",
        "(A1+B1)*C1",
        "IF(A1,,B1)",
        "A1:B2 C1:D2",
        "\"a\"\"b\"&C1",
        "{1,2;3,4}",
        "Table1[[#Headers],[Amount]:[Total]]",
    ] {
        let printed = print_source(source);
        let reparsed = print_source(&printed);
        assert_eq!(reparsed, printed, "{source}");
    }
}

#[test]
fn rejects_invalid_numeric_exponents_without_zero_fallback() {
    // Source: Apache POI FormulaParser::parseNumber requires exponent digits.
    // Invalid exponent text must not be silently lowered to numeric zero.
    for source in ["1E+", "1E-", "1E+A1"] {
        assert_eq!(try_print_source(source), None, "{source}");
    }
}

#[test]
fn rejects_failed_compound_parses_without_partial_program() {
    // Source: Apache POI resets parser state on failed range/function branches;
    // LibreOffice keeps failed formula compilation from emitting a printable token stream.
    for source in ["SUM(1,", "{1,2;3}"] {
        assert_eq!(try_print_source(source), None, "{source}");
    }
}

#[test]
fn prints_nested_function_arguments_without_leaking_inner_args() {
    // Source: LibreOffice ScCompiler stores each function token's parameters separately.
    // Nested calls must not make the outer argument span include inner function arguments.
    assert_eq!(
        print_source("ROUND(FORECAST.ETS.ADD(6,N1:N5,M1:M5),12)"),
        "ROUND(FORECAST.ETS.ADD(6,N1:N5,M1:M5),12)"
    );
    assert_eq!(
        print_source("LET(x,1,LET(x,2,x)+x)"),
        "LET(x,1,LET(x,2,x)+x)"
    );
}

#[test]
fn prints_formula_program_with_leading_equals() {
    let program = FormulaProgram::from_source(FormulaSource {
        text: "A1+B1",
        context: Default::default(),
    });
    let options = FormulaPrintOptions {
        include_leading_equals: true,
        ..FormulaPrintOptions::default()
    };
    assert_eq!(program.print_formula(&options).as_deref(), Some("=A1+B1"));
}

#[test]
fn prints_formula_program_structured_reference() {
    let mut builder = FormulaProgramBuilder::new();
    let column = builder.intern("Amount");
    let reference = builder.structured_reference(
        Some("Table1"),
        FormulaStructuredReferenceSpecifier::Column(column),
    );
    let program = builder.finish(reference);

    assert_eq!(
        program
            .print_formula(&FormulaPrintOptions::default())
            .as_deref(),
        Some("Table1[Amount]")
    );
}

#[test]
fn prints_formula_program_structured_reference_combination() {
    let mut builder = FormulaProgramBuilder::new();
    let start = builder.intern("Amount");
    let end = builder.intern("Total");
    let parts = [
        FormulaStructuredReferencePart::Item(FormulaStructuredReferenceItem::Headers),
        FormulaStructuredReferencePart::ColumnRange { start, end },
    ];
    let span = builder.structured_reference_parts(&parts);
    let reference = builder.structured_reference(
        Some("Table1"),
        FormulaStructuredReferenceSpecifier::Combination(span),
    );
    let program = builder.finish(reference);

    assert_eq!(
        program
            .print_formula(&FormulaPrintOptions::default())
            .as_deref(),
        Some("Table1[[#Headers],[Amount]:[Total]]")
    );
}

#[test]
fn formula_program_editor_exposes_stable_entry_point() {
    let mut builder = FormulaProgramBuilder::new();
    let first = builder.integer(1);
    let second = builder.integer(2);
    let mut program = builder.finish(first);

    assert_eq!(
        program.edit().replace_root(Some(first)),
        Ok(FormulaEditStatus::Unchanged)
    );
    assert_eq!(
        program.edit().replace_root(Some(second)),
        Ok(FormulaEditStatus::Changed)
    );
    assert_eq!(
        program
            .print_formula(&FormulaPrintOptions::default())
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        program.apply_edit(FormulaEditOp::InsertRows {
            sheet: SheetId(0),
            row: 0,
            count: 1,
        }),
        Err(FormulaEditError::UnsupportedOperation)
    );
    assert_eq!(
        program
            .edit()
            .replace_root(Some(ooxmlsdk_formula::program::FormulaExprId(99))),
        Err(FormulaEditError::InvalidExpression)
    );

    let _future_reference_rewrite_shape = FormulaEditOp::Copy {
        source: FormulaEditRange {
            sheet: SheetId(0),
            range: CellRange::new(
                CellAddress { column: 0, row: 0 },
                CellAddress { column: 0, row: 0 },
            ),
        },
        target: FormulaEditRange {
            sheet: SheetId(0),
            range: CellRange::new(
                CellAddress { column: 1, row: 0 },
                CellAddress { column: 1, row: 0 },
            ),
        },
    };
}
