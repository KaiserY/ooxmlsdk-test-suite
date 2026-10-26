use ooxmlsdk::schemas::schemas_microsoft_com_office_visio_2012_main::{
    Connect, Connects, PageContents,
};
use ooxmlsdk_visio::{ConnectError, resolve_connects};

#[test]
fn connections_without_shapes_report_the_first_missing_source() {
    for (from_sheets, expected) in [
        (vec![], Ok(Vec::new())),
        (vec![17], Err(ConnectError::MissingFrom(17))),
        (vec![17, 18], Err(ConnectError::MissingFrom(17))),
    ] {
        let page = PageContents {
            connects: Some(Connects {
                connect: from_sheets
                    .iter()
                    .map(|&from_sheet| Connect {
                        from_sheet,
                        to_sheet: 99,
                        ..Default::default()
                    })
                    .collect(),
            }),
            ..Default::default()
        };
        assert_eq!(resolve_connects(&page), expected);
    }
}
