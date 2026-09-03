use block_native::{
    model::AssetKind,
    package::{build_package, read_package, PackedAsset},
};

#[test]
fn package_round_trips_program_and_assets() {
    let program = b"BLK2-demo".to_vec();
    let bytes = build_package(
        &program,
        &[
            PackedAsset {
                name: "cat".into(),
                kind: AssetKind::Image,
                bytes: vec![1, 2, 3, 4],
            },
            PackedAsset {
                name: "pop".into(),
                kind: AssetKind::Sound,
                bytes: vec![9, 8, 7],
            },
        ],
    )
    .unwrap();

    assert!(bytes.starts_with(b"PK"));
    let loaded = read_package(&bytes).unwrap();
    assert_eq!(loaded.program, program);
    assert_eq!(loaded.assets.get("cat").unwrap().bytes, vec![1, 2, 3, 4]);
    assert_eq!(loaded.assets.get("cat").unwrap().kind, AssetKind::Image);
    assert_eq!(loaded.assets.get("pop").unwrap().kind, AssetKind::Sound);
}

#[test]
fn package_rejects_unsafe_or_duplicate_asset_names() {
    let bad = build_package(
        b"program",
        &[PackedAsset {
            name: "../cat".into(),
            kind: AssetKind::Image,
            bytes: vec![],
        }],
    );
    assert!(bad.is_err());

    let duplicate = build_package(
        b"program",
        &[
            PackedAsset {
                name: "x".into(),
                kind: AssetKind::Image,
                bytes: vec![1],
            },
            PackedAsset {
                name: "x".into(),
                kind: AssetKind::Sound,
                bytes: vec![2],
            },
        ],
    );
    assert!(duplicate.is_err());
}
