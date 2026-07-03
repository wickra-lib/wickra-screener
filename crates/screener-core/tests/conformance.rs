//! Conformance: the serde contract is the cross-language boundary, so every enum
//! variant must survive a JSON round-trip and carry the exact internal tag, and
//! `Expr::key` must render the exact deterministic keys used in `values`.

use std::collections::BTreeMap;

use screener_core::{
    scan_batch, Candle, Comparator, Condition, CsMetric, Expr, PriceField, RankSpec, ScanSpec,
};

/// Serialize, deserialize, and assert the value survives unchanged.
fn roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize");
    let back: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(&back, value, "round-trip changed the value: {json}");
}

/// The internal tag (`kind` for `Expr`, `type` for `Condition`) of a value.
fn tag(value: &impl serde::Serialize, key: &str) -> String {
    let v = serde_json::to_value(value).unwrap();
    v.get(key)
        .and_then(|t| t.as_str())
        .unwrap_or_else(|| panic!("missing {key} tag"))
        .to_string()
}

#[test]
fn expr_variants_round_trip_with_kind_tag() {
    let cst = Expr::Const { value: 1.5 };
    roundtrip(&cst);
    assert_eq!(tag(&cst, "kind"), "const");

    for field in [
        PriceField::Open,
        PriceField::High,
        PriceField::Low,
        PriceField::Close,
        PriceField::Volume,
    ] {
        let price = Expr::Price { field };
        roundtrip(&price);
        assert_eq!(tag(&price, "kind"), "price");
    }

    let ind = Expr::Indicator {
        name: "Rsi".into(),
        params: vec![14.0],
        field: None,
    };
    roundtrip(&ind);
    assert_eq!(tag(&ind, "kind"), "indicator");

    let ind_field = Expr::Indicator {
        name: "Macd".into(),
        params: vec![12.0, 26.0, 9.0],
        field: Some("hist".into()),
    };
    roundtrip(&ind_field);
}

#[test]
fn condition_variants_round_trip_with_type_tag() {
    let close = Expr::Price {
        field: PriceField::Close,
    };
    let cmp = Condition::Cmp {
        left: close.clone(),
        op: Comparator::Gt,
        right: Expr::Const { value: 10.0 },
    };
    roundtrip(&cmp);
    assert_eq!(tag(&cmp, "type"), "cmp");

    let cs = Condition::CrossSection {
        expr: Expr::Indicator {
            name: "Roc".into(),
            params: vec![10.0],
            field: None,
        },
        metric: CsMetric::Rank,
        op: Comparator::Le,
        value: 3.0,
    };
    roundtrip(&cs);
    assert_eq!(tag(&cs, "type"), "cross_section");

    let breadth = Condition::Breadth {
        inner: Box::new(cmp.clone()),
        op: Comparator::Ge,
        ratio: 0.4,
    };
    roundtrip(&breadth);
    assert_eq!(tag(&breadth, "type"), "breadth");

    let all = Condition::All {
        conditions: vec![cmp.clone(), cs.clone()],
    };
    roundtrip(&all);
    assert_eq!(tag(&all, "type"), "all");

    let any = Condition::Any {
        conditions: vec![cmp.clone(), cs],
    };
    roundtrip(&any);
    assert_eq!(tag(&any, "type"), "any");

    let not = Condition::Not {
        condition: Box::new(cmp),
    };
    roundtrip(&not);
    assert_eq!(tag(&not, "type"), "not");
}

#[test]
fn comparators_serialize_snake_case() {
    let cases = [
        (Comparator::Gt, "\"gt\""),
        (Comparator::Ge, "\"ge\""),
        (Comparator::Lt, "\"lt\""),
        (Comparator::Le, "\"le\""),
        (Comparator::Eq, "\"eq\""),
        (Comparator::CrossesAbove, "\"crosses_above\""),
        (Comparator::CrossesBelow, "\"crosses_below\""),
    ];
    for (value, json) in cases {
        assert_eq!(serde_json::to_string(&value).unwrap(), json);
        roundtrip(&value);
    }
}

#[test]
fn cs_metrics_serialize_snake_case() {
    let cases = [
        (CsMetric::Rank, "\"rank\""),
        (CsMetric::PercentileRank, "\"percentile_rank\""),
        (CsMetric::ZScore, "\"z_score\""),
    ];
    for (value, json) in cases {
        assert_eq!(serde_json::to_string(&value).unwrap(), json);
        roundtrip(&value);
    }
}

#[test]
fn price_fields_serialize_snake_case() {
    let cases = [
        (PriceField::Open, "\"open\""),
        (PriceField::High, "\"high\""),
        (PriceField::Low, "\"low\""),
        (PriceField::Close, "\"close\""),
        (PriceField::Volume, "\"volume\""),
    ];
    for (value, json) in cases {
        assert_eq!(serde_json::to_string(&value).unwrap(), json);
        roundtrip(&value);
    }
}

#[test]
fn expr_key_snapshots() {
    assert_eq!(Expr::Const { value: 30.0 }.key(), "const(30)");
    assert_eq!(Expr::Const { value: 1.5 }.key(), "const(1.5)");
    assert_eq!(
        Expr::Price {
            field: PriceField::Close
        }
        .key(),
        "price.close"
    );
    assert_eq!(
        Expr::Indicator {
            name: "Rsi".into(),
            params: vec![14.0],
            field: None,
        }
        .key(),
        "Rsi(14)"
    );
    assert_eq!(
        Expr::Indicator {
            name: "Macd".into(),
            params: vec![12.0, 26.0, 9.0],
            field: Some("hist".into()),
        }
        .key(),
        "Macd(12,26,9).hist"
    );
}

#[test]
fn rank_spec_defaults_desc_false() {
    let spec: RankSpec =
        serde_json::from_str(r#"{"by":{"kind":"price","field":"close"}}"#).unwrap();
    assert!(!spec.desc);
    roundtrip(&spec);
}

/// A rising price path, long enough to warm any indicator used here.
fn candles(n: usize) -> Vec<Candle> {
    (0..n)
        .map(|i| {
            let c = 100.0 + i as f64;
            Candle {
                time: i64::try_from(i).unwrap(),
                open: c,
                high: c,
                low: c,
                close: c,
                volume: 1000.0,
            }
        })
        .collect()
}

fn data() -> BTreeMap<String, Vec<Candle>> {
    let mut m = BTreeMap::new();
    m.insert("AAA".to_string(), candles(40));
    m
}

#[test]
fn unknown_indicator_is_an_error() {
    let spec: ScanSpec = serde_json::from_str(
        r#"{"universe":["AAA"],"condition":{"type":"cmp",
        "left":{"kind":"indicator","name":"NopeIndicator","params":[5]},
        "op":"gt","right":{"kind":"const","value":0}}}"#,
    )
    .unwrap();
    assert!(scan_batch(&data(), &spec).is_err());
}

#[test]
fn unknown_field_yields_no_value_not_an_error() {
    // A typo'd sub-output field is tolerated: the indicator resolves, the field
    // is absent, and the comparison is simply false — the scan still succeeds.
    let spec: ScanSpec = serde_json::from_str(
        r#"{"universe":["AAA"],"condition":{"type":"cmp",
        "left":{"kind":"indicator","name":"Rsi","params":[14],"field":"bogus"},
        "op":"gt","right":{"kind":"const","value":0}}}"#,
    )
    .unwrap();
    let report = scan_batch(&data(), &spec).expect("scan succeeds");
    assert_eq!(report.scanned, 1);
    assert!(report.matches.is_empty());
}
