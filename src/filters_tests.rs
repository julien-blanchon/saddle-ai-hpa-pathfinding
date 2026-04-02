use super::{AreaMask, AreaTypeId, PathFilterLibrary, PathFilterProfile};

#[test]
fn filter_library_assigns_stable_ids() {
    let mut library = PathFilterLibrary::default();
    let default_id = library.register(PathFilterProfile::named("default"));
    let mud_id =
        library.register(PathFilterProfile::named("mud").with_area_cost(AreaTypeId(2), 4.0));

    assert_eq!(default_id.0, 0);
    assert!(mud_id.0 > default_id.0);
    assert_eq!(
        library.get(mud_id).unwrap().multiplier_for(AreaTypeId(2)),
        4.0
    );
}

#[test]
fn area_mask_helpers_are_predictable() {
    let a = AreaMask::from_bit(1);
    let b = AreaMask::from_bit(2);
    let combined = AreaMask(a.0 | b.0);

    assert!(combined.contains(a));
    assert!(combined.intersects(b));
    assert!(!a.intersects(b));
}
