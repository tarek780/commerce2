use category_service::graphql::pagination::{PageInput, ProductFilterInput};
use category_service::helpers::cursor::Cursor;

#[test]
fn default_limit_is_20() {
    assert_eq!(PageInput::default().limit(), 20);
}

#[test]
fn limit_is_clamped() {
    assert_eq!(PageInput { first: Some(0), after: None }.limit(), 1);
    assert_eq!(PageInput { first: Some(-5), after: None }.limit(), 1);
    assert_eq!(PageInput { first: Some(1000), after: None }.limit(), 100);
    assert_eq!(PageInput { first: Some(50), after: None }.limit(), 50);
}

#[test]
fn cursor_from_after() {
    let after = Cursor { id: 99 }.encode();
    let p = PageInput { first: Some(10), after: Some(after) };
    assert_eq!(p.cursor().unwrap().id, 99);
}

#[test]
fn invalid_after_yields_none() {
    let p = PageInput { first: None, after: Some("bad".into()) };
    assert!(p.cursor().is_none());
}

#[test]
fn filter_default_is_all_none() {
    let f = ProductFilterInput::default();
    assert!(f.category_id.is_none());
    assert!(f.brand.is_none());
    assert!(f.search.is_none());
    assert!(f.is_active.is_none());
    assert!(f.min_price.is_none());
    assert!(f.max_price.is_none());
}