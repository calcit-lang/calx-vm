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
      Cirru::leaf("a"),
      Cirru::leaf("b"),
      Cirru::List(vec![
        Cirru::leaf("c"),
        Cirru::leaf("d"),
        Cirru::List(vec![Cirru::leaf("e"), Cirru::leaf("f"),])
      ])
    ]))?),
    Cirru::List(vec!(
      Cirru::List(vec![Cirru::leaf("e"), Cirru::leaf("f")]),
      Cirru::List(vec![Cirru::leaf("c"), Cirru::leaf("d")]),
      Cirru::List(vec![Cirru::leaf("a"), Cirru::leaf("b")])
    ))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::leaf("block"),
      Cirru::List(vec![
        Cirru::leaf("c"),
        Cirru::leaf("d"),
        Cirru::List(vec![Cirru::leaf("e"), Cirru::leaf("f"),])
      ])
    ]))?),
    Cirru::List(vec!(Cirru::List(vec![
      Cirru::leaf("block"),
      Cirru::List(vec![Cirru::leaf("e"), Cirru::leaf("f")]),
      Cirru::List(vec![Cirru::leaf("c"), Cirru::leaf("d")])
    ]),))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::leaf("loop"),
      Cirru::List(vec![
        Cirru::leaf("c"),
        Cirru::leaf("d"),
        Cirru::List(vec![Cirru::leaf("e"), Cirru::leaf("f"),])
      ])
    ]))?),
    Cirru::List(vec!(Cirru::List(vec![
      Cirru::leaf("loop"),
      Cirru::List(vec![Cirru::leaf("e"), Cirru::leaf("f")]),
      Cirru::List(vec![Cirru::leaf("c"), Cirru::leaf("d")])
    ]),))
  );

  Ok(())
}
