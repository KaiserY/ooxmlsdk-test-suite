use ooxmlsdk_formula::program::{
    FormulaPrintOptions, FormulaProgram, FormulaProgramBuilder, FormulaStructuredReferenceSpecifier,
};
use ooxmlsdk_formula::source::FormulaSource;

fn print_source(formula: &str) -> String {
    FormulaProgram::from_source(FormulaSource {
        text: formula,
        context: Default::default(),
    })
    .print_formula(&FormulaPrintOptions::default())
    .expect("formula program should print")
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
        ("{1,2;3,4}", "{1,2;3,4}"),
        ("Sheet1!A1", "Sheet1!A1"),
        ("'My Sheet'!$A$1:$B$2", "'My Sheet'!$A$1:$B$2"),
        ("#DIV/0!", "#DIV/0!"),
    ] {
        assert_eq!(print_source(source), expected, "{source}");
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
