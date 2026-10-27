
use crate::graphql::pagination::ProductFilterInput;

pub fn build_redis_query(f: &ProductFilterInput) -> String { 
/* ... same body ... */ 

}

use category_service::graphql::pagination::ProductFilterInput;
use category_service::helpers::redis_query::build_redis_query;

#[test]
fn empty_filter_is_star() {
    assert_eq!(build_redis_query(&ProductFilterInput::default()), "*");
}

#[test]
fn category_and_brand() {
    let f = ProductFilterInput {
        category_id: Some(3),
        brand: Some("Acme".into()),
        ..Default::default()
    };
    let q = build_redis_query(&f);
    assert!(q.contains("@category_id:[3 3]"));
    assert!(q.contains("@brand:{Acme}"));
}

#[test]
fn active_flag() {
    let t = ProductFilterInput { is_active: Some(true), ..Default::default() };
    assert!(build_redis_query(&t).contains("@is_active:{true}"));
    let f = ProductFilterInput { is_active: Some(false), ..Default::default() };
    assert!(build_redis_query(&f).contains("@is_active:{false}"));
}

#[test]
fn price_range() {
    let f = ProductFilterInput {
        min_price: Some(10.0),
        max_price: Some(50.0),
        ..Default::default()
    };
    let q = build_redis_query(&f);
    assert!(q.contains("@price_min:[10"));
    assert!(q.contains("+inf]"));
    assert!(q.contains("[-inf 50"));
}

#[test]
fn search_wildcard() {
    let f = ProductFilterInput { search: Some("sh".into()), ..Default::default() };
    assert!(build_redis_query(&f).contains("@name:sh*"));
}

#[test]
fn combined() {
    let f = ProductFilterInput {
        category_id: Some(1),
        brand: Some("B".into()),
        is_active: Some(true),
        ..Default::default()
    };
    let q = build_redis_query(&f);
    assert!(q.starts_with('*'));
    assert!(q.contains("@category_id:[1 1]"));
    assert!(q.contains("@brand:{B}"));
    assert!(q.contains("@is_active:{true}"));
}