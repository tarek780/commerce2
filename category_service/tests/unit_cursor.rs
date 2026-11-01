use base64::{Engine as _, engine::general_purpose};
use category_service::error::AppError;
use category_service::helpers::cursor::{Cursor, decode_required};
use uuid::Uuid;

#[test]
fn roundtrip() {
    let id = Uuid::now_v7();

    let c = Cursor { id };
    assert_eq!(Cursor::decode(&c.encode()).unwrap().id, id);
}

#[test]
fn url_safe_alphabet() {
    let id = Uuid::now_v7();
    let s = Cursor { id }.encode();
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
    let id = Uuid::now_v7();
    let ok = decode_required(&Cursor { id }.encode()).unwrap();
    assert_eq!(ok.id, id);
}
