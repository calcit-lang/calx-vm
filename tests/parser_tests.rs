use cirru_parser::Cirru;

use calx_vm::extract_nested;

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
