use ooxmlsdk_pdf_test::{OfficeGoldenCase, VisualTolerance, compare_office_golden};

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf104015() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf104015",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf104015.pptx",
            source_sha256: "5a986fa43afc51500616b5561202faaa8250afe435cb34758abb023569fa9a8c",
            golden_sha256: "4cdb7069fc18046bca8687e728bf92566d903ea1ad4f83ede97c3aabd2b568fd",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf105150() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf105150",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf105150.pptx",
            source_sha256: "fde2b32be9f920b32e95b6ad4504d438808e1ff5406ed5f791434c1d4e500c04",
            golden_sha256: "49861f5ecbed810bce743359b4d5fd6607bb1aabbd1b275ae540758788153284",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf127964() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf127964",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf127964.pptx",
            source_sha256: "d91b5d5379029d6e68478c9e9d8c477b3b0530e5c96c50a902447a41378f01cb",
            golden_sha256: "dc71063e98d5c1549aac7793a5fecd539e5418b161a064fc8a423c1458be5eea",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf93868() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf93868",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf93868.pptx",
            source_sha256: "afc3699ff09d898e9e07e004fdd721ff8532b2915dc6abffab67bed80dbf67d6",
            golden_sha256: "aa44e46f96958593de4bca2927b386822e529b516caf4de7f54d1d733e9a3b24",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf109067() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf109067",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf109067.pptx",
            source_sha256: "ce64765c0cd0149a83dcc2a2664ce6b6ac213394dd3c9d7b353624dad95c1a14",
            golden_sha256: "b90ea593ddea83309eb2bb0765519160ec94cdcbfd410713a594bda9c2e9e566",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf109187() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf109187",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf109187.pptx",
            source_sha256: "5c4f310debee15f456feecf757b2bbdad59af4e9d99f2b9f784e8a67d38c27cf",
            golden_sha256: "0db412d3a3e283ee0e033567ed7c7b51632545d87696982a9eca6dc2ac891f31",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf111518() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf111518",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf111518.pptx",
            source_sha256: "01d75f39e5b711d4b259503334029fd861698cde74321ebe853f22325af89166",
            golden_sha256: "16f6570bd500fca25b93d0befd00cb40ac288b4fd2148dbf2db234e7be178caa",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf111786() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf111786",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf111786.pptx",
            source_sha256: "b4e8fb935024deefa12c1ce943748cc9db276b16b99fb7837ba02d63c78d94ee",
            golden_sha256: "8c24b79443dc4cd50dca38684378d93ac4360feced76b6ab41f2060341a18756",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf111789() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf111789",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf111789.pptx",
            source_sha256: "bf8f2efc02bc4a8b5c41066fdc8a7a7569283005d272d4c8abd4f08807039898",
            golden_sha256: "a0c0d68a20a753e6fa7e2fc4024c0bd96f2895ffd66a396d993380b83d269c6b",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf111863() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf111863",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf111863.pptx",
            source_sha256: "47e8ec870d87585f846ce47fb39dad3c075568419cacfaf3ad88887af9c589f1",
            golden_sha256: "af3fe87ca14e058c3676378be632dfc7bb4e403f2072a7704371aabe0965baf9",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf111884() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf111884",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf111884.pptx",
            source_sha256: "8f59ee500b534e63dcb590e046512d887161da7c2d7ad42f984138653828d99b",
            golden_sha256: "edb532f3a0fcf337591e0b0abac17523a72abccf760156ef29220634f2dd5d96",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112086() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112086",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112086.pptx",
            source_sha256: "428db337e3716263e656dfdb2d8265fb8121a047c0630229e1c0937e8cf50580",
            golden_sha256: "703e932e27ccf43c73380778fbc5de15c7d8e74c49c3a59877b1c81b9153c0f1",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112088() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112088",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112088.pptx",
            source_sha256: "e716846104d042a5bdf80de1aca6a3ec639e0046882d61d1e614031c83712c2f",
            golden_sha256: "41755182d35058220f39612ac70a4420061257a5de13f72894be4b2ed09e7724",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112089() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112089",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112089.pptx",
            source_sha256: "76a2c841fa085e991c53ba5ddcc4a206ee93a98ca87f13c656ab6408bfb4c6c1",
            golden_sha256: "c7bd136901ba4590c9b37465c88e7f434d197ac27d8a3ab6240762b70a9403fb",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112209() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112209",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112209.pptx",
            source_sha256: "d0085b0f0daa6d3e8a6e5a6329d0fa9a32057f1032296c6029bda01f42e6621a",
            golden_sha256: "8c0bfa911ae2dd4cff0384e823065d8b233b4a3d0ec62a0e61eb2d4e346c1890",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112280() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112280",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112280.pptx",
            source_sha256: "31339f5e0e6c023bfa6e7699528f479e4c1b1464c4eae96c70f0a39dd7c4e2f4",
            golden_sha256: "5e32162d442ea04716524897d01873696b71d91a73cb8e41f57354047fc3348f",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112333() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112333",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112333.pptx",
            source_sha256: "e5dd6520572385aae80159bf7d7ede0e7368f7bf375fc72a652ecfd2e7c0f03d",
            golden_sha256: "b09f42c86e5a0be75088fd4e9a5651c431c81fa3e1f1ae97db54803186c95d11",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112334() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112334",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112334.pptx",
            source_sha256: "43ad429a76716e78d61497b4740ced9f0273a6e9d553cac48d0dacf9239155f7",
            golden_sha256: "471604e3ff78b34b38aa13a65a28ff5307700810b4e91ad750b98b6fd0f6470c",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf112633() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf112633",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf112633.pptx",
            source_sha256: "a713241c150bf7ec1dc85ca37683a47ee37a98af0bba2b803bac4a188c40e344",
            golden_sha256: "8dbd2187eed15cb1ff11b36acaff4270af0dd6d0383d5ebaff1eb2423cab7203",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}

#[test]
#[ignore = "run Office golden corpus cases explicitly"]
fn office_golden_libreoffice_sd_qa_unit_data_pptx_tdf113163() {
    let report = compare_office_golden(
        OfficeGoldenCase {
            id: "libreoffice_sd_qa_unit_data_pptx_tdf113163",
            corpus: "LibreOffice",
            source: "sd/qa/unit/data/pptx/tdf113163.pptx",
            source_sha256: "8e746b8e6017f373af2233d4fd66807bef7fa79b38514cb8b1435edd96426f84",
            golden_sha256: "d7bc34f9783dbf2bdf71fa4de8d03b1b2464ccc56eacd34c03c06ecd6145418e",
            environment_id: "238d45fa17f25b86fbd61ee81bb755cb9692dbd5ba881afdea771268e08e9157",
            ui_language: "zh-CN",
        },
        VisualTolerance::OFFICE_FIXED_OUTPUT,
    )
    .unwrap();

    assert_eq!(report.page_diffs.len(), 1);
}
