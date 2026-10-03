use workshop_core::{decoder, setup, storage};
#[test]
fn display_and_write_unicode_save_names() {
    use workshop_core::sii::{quoted, unquote};
    let name = "我的卡车 \"test\" C:\\games";
    assert_eq!(unquote(&quoted(name).unwrap()), name);
    assert_eq!(unquote(r#""\xe4\xb8\xad\xe6\x96\x87""#), "中文");
}
#[test]
fn independent_ordinal_dictionaries_preserved() {
    let text = decoder::decode_direct(include_bytes!("fixtures/two-ordinal-fields.bsii")).unwrap();
    assert!(text.contains("first: alpha"));
    assert!(text.contains("second: beta"));
}
#[test]
fn unicode_strings_match_reference_sii_encoding() {
    let text = decoder::decode_direct(include_bytes!("fixtures/unicode-strings.bsii")).unwrap();
    assert!(text.contains(r#"name: "\xe4\xb8\xad\xe6\x96\x87""#));
    assert!(text.contains(r#"names[0]: "\xe4\xb8\xad\xe6\x96\x87""#));
}
#[test]
fn native_encrypted_roundtrip_without_dll() {
    use cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
    use std::io::Write;
    let plain =
        b"SiiNunit\n{\nsave_container : info {\n name: test_native\n dependencies: 0\n}\n}\n";
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(plain).unwrap();
    let compressed = z.finish().unwrap();
    let key: [u8; 32] = [
        0x2a, 0x5f, 0xcb, 0x17, 0x91, 0xd2, 0x2f, 0xb6, 0x02, 0x45, 0xb3, 0xd8, 0x36, 0x9e, 0xd0,
        0xb2, 0xc2, 0x73, 0x71, 0x56, 0x3f, 0xbf, 0x1f, 0x3c, 0x9e, 0xdf, 0x6b, 0x11, 0x82, 0x5a,
        0x5d, 0x0a,
    ];
    let iv = [7u8; 16];
    let mut padded = compressed.clone();
    padded.resize(compressed.len() + 16, 0);
    let bytes = cbc::Encryptor::<aes::Aes256>::new(&key.into(), &iv.into())
        .encrypt_padded_mut::<Pkcs7>(&mut padded, compressed.len())
        .unwrap();
    let mut file = b"ScsC".to_vec();
    file.extend([0u8; 32]);
    file.extend(iv);
    file.extend((plain.len() as u32).to_le_bytes());
    file.extend(bytes);
    assert_eq!(decoder::decode_direct(&file).unwrap().as_bytes(), plain);
    assert_eq!(decoder::decode_direct(plain).unwrap().as_bytes(), plain);
}
#[test]
fn truncated_and_unknown_formats_fail() {
    for b in [b"ScsC".as_slice(), b"BSII", b"garbage", b""] {
        assert!(decoder::decode_direct(b).is_err());
    }
}
#[test]
fn mods_and_unknown_dependencies_rejected() {
    for dependency in [
        "mod|custom|Example",
        "package|x|Example",
        "workshop|123|Example",
        "unknown",
    ] {
        let info=format!("SiiNunit {{\nsave_container : info {{\n dependencies: 1\n dependencies[0]: \"{dependency}\"\n}}\n}}\n");
        assert!(storage::has_mod_dependencies(&info).unwrap());
    }
    let info="SiiNunit {\nsave_container : info {\n dependencies: 2\n dependencies[0]: \"dlc|eut2|Official\"\n dependencies[1]: \"rdlc|eut2|Official\"\n}\n}\n";
    assert!(!storage::has_mod_dependencies(info).unwrap());
    assert!(storage::has_mod_dependencies("").is_err());
}
#[test]
fn no_dependencies_outside_game_required() {
    let s = storage::Settings::default();
    assert_eq!(s.onboarding_version, 0);
    assert!(s.extractor.ends_with("scs_extractor.exe"));
    assert_eq!(setup::SETUP_VERSION, 1);
}
