use emfsdk::common::SdkEnumValue;
use emfsdk::emf::{
    EMF_SIGNATURE, EmfHeader, EmfRecordData, EmfRecordType, EmrPointTypeValue, EmrPolyDrawL,
};
use emfsdk::types::{PointL, RectL, SizeL};
use emfsdk::wmf::{WmfRecordData, WmfRecordFunction};
use emfsdk::{EmfRecord, WmfMetafile, WmfPlaceableHeader, WmfRecord};

#[test]
fn poi_placeable_wmf_rejects_zero_units_per_inch() {
    let valid = minimal_placeable_wmf(72);
    WmfMetafile::from_bytes(&valid).expect("valid placeable WMF should parse");

    let invalid = minimal_placeable_wmf(0);
    assert!(
        WmfMetafile::from_bytes(&invalid).is_err(),
        "Apache POI rejects placeable WMF files with zero units per inch"
    );
}

#[test]
fn poi_create_region_rejects_invalid_scan_count() {
    let create_region = create_region_record();
    let parsed = create_region.parse_data().expect("valid META_CREATEREGION");
    let WmfRecordData::CreateRegion(region) = parsed else {
        panic!("expected META_CREATEREGION");
    };
    assert_eq!(region.scan_count as usize, region.scans.len());
    assert_eq!(region.max_scan, 2);

    let mut invalid = create_region.clone();
    invalid.data[2..4].copy_from_slice(&5i16.to_le_bytes());
    assert!(
        invalid.parse_data().is_err(),
        "Apache POI treats inconsistent META_CREATEREGION scan counts as invalid"
    );
}

#[test]
fn poi_polydraw_rejects_invalid_bezier_sequence() {
    let bounds = RectL {
        left: 0,
        top: 0,
        right: 10,
        bottom: 10,
    };
    let invalid = EmfRecordData::PolyDraw(EmrPolyDrawL {
        bounds,
        points: vec![
            PointL { x: 1, y: 2 },
            PointL { x: 3, y: 4 },
            PointL { x: 5, y: 6 },
        ],
        point_types: point_types(&[0x06, 0x04, 0x04]),
        padding: vec![0],
    });
    assert!(
        invalid.to_record().is_err(),
        "Apache POI rejects EMR_POLYDRAW bezier sequences without three BezierTo points"
    );

    let invalid_record = EmfRecord::new(EmfRecordType::PolyDraw.raw(), {
        let mut data = Vec::new();
        data.extend_from_slice(&bounds.left.to_le_bytes());
        data.extend_from_slice(&bounds.top.to_le_bytes());
        data.extend_from_slice(&bounds.right.to_le_bytes());
        data.extend_from_slice(&bounds.bottom.to_le_bytes());
        data.extend_from_slice(&3u32.to_le_bytes());
        for point in [
            PointL { x: 1, y: 2 },
            PointL { x: 3, y: 4 },
            PointL { x: 5, y: 6 },
        ] {
            data.extend_from_slice(&point.x.to_le_bytes());
            data.extend_from_slice(&point.y.to_le_bytes());
        }
        data.extend_from_slice(&[0x06, 0x04, 0x04, 0x00]);
        data
    });
    assert!(invalid_record.parse_data().is_err());
}

#[test]
fn poi_header_description_validates_bounds_and_utf16_shape() {
    let mut header = header_with_description(
        9,
        88,
        vec![
            b'L', 0, b'o', 0, b'n', 0, b'g', 0, b'N', 0, b'a', 0, b'm', 0, b'e', 0, 0, 0,
        ],
    );
    assert!(header.to_record_data().is_ok());
    assert_eq!(
        header
            .description()
            .expect("valid description bounds")
            .expect("description present")
            .encoded_bytes()
            .expect("raw UTF-16LE bytes"),
        vec![
            b'L', 0, b'o', 0, b'n', 0, b'g', 0, b'N', 0, b'a', 0, b'm', 0, b'e', 0, 0, 0
        ]
    );

    header.description_offset = 999;
    assert!(header.description().is_err());

    let malformed = header_with_description(2, 88, vec![b'N', 0, b'o', 0]);
    let compatible_bytes = malformed
        .to_record_data()
        .expect("compatible parsing preserves the producer bytes");
    let reparsed = EmfHeader::from_record_data(&compatible_bytes).expect("compatible header");
    assert_eq!(reparsed.to_record_data().unwrap(), compatible_bytes);
    assert!(
        reparsed.validate_strict().is_err(),
        "strict validation follows Apache POI and requires null-terminated UTF-16LE"
    );
}

fn minimal_placeable_wmf(inch: u16) -> Vec<u8> {
    let placeable = WmfPlaceableHeader {
        key: 0x9AC6_CDD7,
        handle: 0,
        left: 0,
        top: 0,
        right: 1,
        bottom: 1,
        inch,
        reserved: 0,
        checksum: 0,
    };
    let checksum = placeable.computed_checksum();

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&placeable.key.to_le_bytes());
    bytes.extend_from_slice(&placeable.handle.to_le_bytes());
    bytes.extend_from_slice(&placeable.left.to_le_bytes());
    bytes.extend_from_slice(&placeable.top.to_le_bytes());
    bytes.extend_from_slice(&placeable.right.to_le_bytes());
    bytes.extend_from_slice(&placeable.bottom.to_le_bytes());
    bytes.extend_from_slice(&placeable.inch.to_le_bytes());
    bytes.extend_from_slice(&placeable.reserved.to_le_bytes());
    bytes.extend_from_slice(&checksum.to_le_bytes());

    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&9u16.to_le_bytes());
    bytes.extend_from_slice(&0x0300u16.to_le_bytes());
    bytes.extend_from_slice(&12u32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());

    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

fn create_region_record() -> WmfRecord {
    WmfRecord::new(
        WmfRecordFunction::CreateRegion.raw(),
        [
            0u16.to_le_bytes().as_slice(),
            6i16.to_le_bytes().as_slice(),
            1u32.to_le_bytes().as_slice(),
            34i16.to_le_bytes().as_slice(),
            1i16.to_le_bytes().as_slice(),
            2i16.to_le_bytes().as_slice(),
            0i16.to_le_bytes().as_slice(),
            1i16.to_le_bytes().as_slice(),
            10i16.to_le_bytes().as_slice(),
            11i16.to_le_bytes().as_slice(),
            2u16.to_le_bytes().as_slice(),
            1u16.to_le_bytes().as_slice(),
            2u16.to_le_bytes().as_slice(),
            3u16.to_le_bytes().as_slice(),
            4u16.to_le_bytes().as_slice(),
            2u16.to_le_bytes().as_slice(),
        ]
        .concat(),
    )
}

fn header_with_description(
    description_chars: u32,
    description_offset: u32,
    extension: Vec<u8>,
) -> EmfHeader {
    EmfHeader {
        bounds: RectL {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
        frame: RectL {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        },
        signature: EMF_SIGNATURE,
        version: 0x0001_0000,
        bytes: 0,
        records: 0,
        handles: 0,
        reserved: 0,
        description_chars,
        description_offset,
        palette_entries: 0,
        device: SizeL { cx: 1, cy: 1 },
        millimeters: SizeL { cx: 1, cy: 1 },
        extension,
    }
}

fn point_types(values: &[u8]) -> Vec<EmrPointTypeValue> {
    values
        .iter()
        .copied()
        .map(|value| EmrPointTypeValue::new(value).expect("valid EMR point type"))
        .collect()
}
