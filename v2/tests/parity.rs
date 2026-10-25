use serde_json::Value;
use tezbar_json_to_toon_native::{convert, trim_empty, Error, MAX_INPUT, MAX_NODES};

#[test]
fn immutable_actual_json_parse_and_locked_library_oracle() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("oracle-native.json")).unwrap();
    let mut surrogate_gaps = 0;
    for case in &cases {
        let input = case["input"].as_str().unwrap();
        match convert(input) {
            Ok(output) => {
                assert_eq!(case["ok"], true, "{input:?}");
                assert_eq!(case["output"], output, "{input:?}");
            }
            Err(Error::UnpairedSurrogate) => {
                assert_eq!(case["ok"], true);
                surrogate_gaps += 1;
            }
            Err(error) => assert_eq!(case["ok"], false, "{input:?}: {error}"),
        }
    }
    assert_eq!(cases.len(), 4558);
    assert_eq!(surrogate_gaps, 3, "explicit UTF-16 representation boundary");
}
#[test]
fn limits_reject_whole_input_and_errors_never_include_private_source() {
    assert_eq!(convert(&" ".repeat(MAX_INPUT + 1)), Err(Error::InputLimit));
    assert_eq!(
        convert(&format!("{}0{}", "[".repeat(65), "]".repeat(65))),
        Err(Error::DepthLimit)
    );
    let huge = format!("[{}]", vec!["0"; MAX_NODES].join(","));
    assert_eq!(convert(&huge), Err(Error::NodeLimit));
    assert_eq!(convert("{\"private secret\":"), Err(Error::InvalidJson));
    assert!(!Error::InvalidJson.message().contains("private secret"));
    assert_eq!(
        convert(&format!("{}0{}", "[".repeat(64), "]".repeat(64)))
            .unwrap()
            .lines()
            .count(),
        64
    );
    let wide = format!(
        "{}[{}]{}",
        "[".repeat(62),
        vec!["{}"; 40000].join(","),
        "]".repeat(62)
    );
    assert_eq!(convert(&wide), Err(Error::OutputLimit));
}
#[test]
fn ecmascript_trim_is_not_rust_unicode_trim() {
    assert!(trim_empty("\u{feff}\u{00a0}\u{2028}\u{2007}\t"));
    assert!(!trim_empty("\u{0085}"));
    assert!(!trim_empty("\u{180e}"));
    assert!(convert("\u{feff}1").is_err());
}
