#![cfg(feature = "parts")]

use ooxmlsdk::parts::{
    header_part::HeaderPart, image_part::ImagePart, wordprocessing_document::WordprocessingDocument,
};
use ooxmlsdk::sdk::SdkPackage;

#[test]
fn common_part_methods_remain_available_without_importing_sdk_part() {
    fn assert_method_lookup<P: SdkPackage>(part: &ImagePart, package: &mut P, header: &HeaderPart) {
        let _ = part.path(&*package);
        let _ = part.data(&*package);
        let _ = part.get_id_of_part(&*package, header);
        let _ = part.add_new_part_auto_id::<P, HeaderPart>(package);

        let _ = ImagePart::path(part, &*package);
        let _ = ImagePart::data(part, &*package);
        let _ = ImagePart::get_id_of_part(part, &*package, header);
        let _ = ImagePart::add_new_part_auto_id::<P, HeaderPart>(part, package);
    }

    let _ = assert_method_lookup::<WordprocessingDocument>;
}
