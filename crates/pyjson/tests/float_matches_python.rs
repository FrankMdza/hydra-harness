//! String float conversions recorded with CPython 3.14.7 for build 223.
use hydra_pyjson::{
    json::{Value, loads_str},
    py::float_literal,
};

#[test]
fn float_strings_match_python() {
    let Value::List(rows) = loads_str(include_str!("fixtures/float.json")).unwrap() else {
        panic!("list");
    };
    for row in rows {
        let Value::List(cells) = row else {
            panic!("row")
        };
        let Value::Str(input) = &cells[0] else {
            panic!("input")
        };
        let got = float_literal(&input.lossy()).map(|value| {
            if value.is_nan() {
                "nan".to_string()
            } else {
                format!("{:016x}", value.to_bits())
            }
        });
        let expected = match &cells[1] {
            Value::Null => None,
            Value::Str(s) => Some(s.lossy()),
            _ => panic!("expected"),
        };
        assert_eq!(got, expected, "{input:?}");
    }
}
