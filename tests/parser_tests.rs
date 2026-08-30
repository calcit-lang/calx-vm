use cirru_parser::Cirru;

use calx_vm::{extract_nested, parse_function};
use cirru_parser::parse;

/// extracting nested expression inside
/// block and loop are special need to handle
#[test]
fn test_extracting() -> Result<(), String> {
  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![Cirru::leaf("a"), Cirru::leaf("b")]))?),
    Cirru::List(vec!(Cirru::List(vec![Cirru::leaf("a"), Cirru::leaf("b")])))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::leaf("a"),
      Cirru::leaf("b"),
      Cirru::List(vec![Cirru::leaf("c"), Cirru::leaf("d"),])
    ]))?),
    Cirru::List(vec!(
      Cirru::List(vec![Cirru::leaf("c"), Cirru::leaf("d")]),
      Cirru::List(vec![Cirru::leaf("a"), Cirru::leaf("b")])
    ))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      "a".into(),
      "b".into(),
      Cirru::List(vec![Cirru::leaf("c"), Cirru::leaf("d"), Cirru::List(vec!["e".into(), "f".into(),])])
    ]))?),
    Cirru::List(vec!(
      Cirru::List(vec!["e".into(), "f".into()]),
      Cirru::List(vec!["c".into(), "d".into()]),
      Cirru::List(vec!["a".into(), "b".into()])
    ))
  );

  Ok(())
}

fn parse_single_function(source: &str) -> Result<(), String> {
  let nodes = parse(source).map_err(|error| error.to_string())?;
  let Some(Cirru::List(function)) = nodes.first() else {
    return Err("expected one function".to_string());
  };
  parse_function(function).map(|_| ())
}

#[test]
fn malformed_structured_forms_return_errors_instead_of_panicking() {
  for (source, expected) in [
    ("fn main ()\n  block", "block expected a type signature"),
    ("fn main ()\n  loop", "loop expected a type signature"),
  ] {
    let error = parse_single_function(source).expect_err("malformed guest syntax must be rejected");
    assert!(error.contains(expected), "expected {expected:?}, got {error:?}");
  }

  let empty_do = vec![
    Cirru::leaf("fn"),
    Cirru::leaf("main"),
    Cirru::List(vec![]),
    Cirru::List(vec![
      Cirru::leaf("if"),
      Cirru::List(vec![Cirru::leaf("->")]),
      Cirru::List(vec![]),
      Cirru::List(vec![Cirru::leaf("do")]),
    ]),
  ];
  let error = parse_function(&empty_do).expect_err("an empty do expression must be rejected");
  assert!(error.contains("expected `do`, got an empty expression"), "{error}");
}
