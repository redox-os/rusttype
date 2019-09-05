use rusttype::*;

#[test]
fn static_lazy_shared_bytes() {
    use once_cell::sync::Lazy;
    static FONT_BYTES: Lazy<Vec<u8>> = Lazy::new(|| vec![0, 1, 2, 3]);

    let shared_bytes: SharedBytes<'static> = (&*FONT_BYTES).into();
    assert_eq!(&*shared_bytes, &[0, 1, 2, 3]);
}
