use category_service::error::AppError;
use category_service::helpers::cursor::{decode_required, Cursor};
use base64::{engine::general_purpose, Engine as _};

#[test]
fn roundtrip() {
    let c = Cursor { id: 42 };
    assert_eq!(Cursor::decode(&c.encode()).unwrap().id, 42);
}

#[test]
fn url_safe_alphabet() {
    let s = Cursor { id: i64::MAX }.encode();
    assert!(!s.contains('+') && !s.contains('/') && !s.contains('='));
}

#[test]
fn invalid_inputs_yield_none() {
    assert!(Cursor::decode("!!!not-base64!!!").is_none());
    assert!(Cursor::decode("").is_none());
    let not_number = general_purpose::URL_SAFE_NO_PAD.encode("hello");
    assert!(Cursor::decode(&not_number).is_none());
}

#[test]
fn decode_required_error_variant() {
    let err = decode_required("garbage").unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[test]
fn decode_required_ok() {
    let ok = decode_required(&Cursor { id: 7 }.encode()).unwrap();
    assert_eq!(ok.id, 7);
}