use cirru_parser::Cirru;

use calx_vm::extract_nested;

/// extracting nested expression inside
/// block and loop are special need to handle
#[test]
fn test_extracting() -> Result<(), String> {
  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::Leaf(String::from("a")),
      Cirru::Leaf(String::from("b"))
    ]))?),
    Cirru::List(vec!(Cirru::List(vec![
      Cirru::Leaf(String::from("a")),
      Cirru::Leaf(String::from("b"))
    ])))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::Leaf(String::from("a")),
      Cirru::Leaf(String::from("b")),
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d")),
      ])
    ]))?),
    Cirru::List(vec!(
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d"))
      ]),
      Cirru::List(vec![
        Cirru::Leaf(String::from("a")),
        Cirru::Leaf(String::from("b"))
      ])
    ))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::Leaf(String::from("a")),
      Cirru::Leaf(String::from("b")),
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d")),
        Cirru::List(vec![
          Cirru::Leaf(String::from("e")),
          Cirru::Leaf(String::from("f")),
        ])
      ])
    ]))?),
    Cirru::List(vec!(
      Cirru::List(vec![
        Cirru::Leaf(String::from("e")),
        Cirru::Leaf(String::from("f"))
      ]),
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d"))
      ]),
      Cirru::List(vec![
        Cirru::Leaf(String::from("a")),
        Cirru::Leaf(String::from("b"))
      ])
    ))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::Leaf(String::from("block")),
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d")),
        Cirru::List(vec![
          Cirru::Leaf(String::from("e")),
          Cirru::Leaf(String::from("f")),
        ])
      ])
    ]))?),
    Cirru::List(vec!(Cirru::List(vec![
      Cirru::Leaf(String::from("block")),
      Cirru::List(vec![
        Cirru::Leaf(String::from("e")),
        Cirru::Leaf(String::from("f"))
      ]),
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d"))
      ])
    ]),))
  );

  assert_eq!(
    Cirru::List(extract_nested(&Cirru::List(vec![
      Cirru::Leaf(String::from("loop")),
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d")),
        Cirru::List(vec![
          Cirru::Leaf(String::from("e")),
          Cirru::Leaf(String::from("f")),
        ])
      ])
    ]))?),
    Cirru::List(vec!(Cirru::List(vec![
      Cirru::Leaf(String::from("loop")),
      Cirru::List(vec![
        Cirru::Leaf(String::from("e")),
        Cirru::Leaf(String::from("f"))
      ]),
      Cirru::List(vec![
        Cirru::Leaf(String::from("c")),
        Cirru::Leaf(String::from("d"))
      ])
    ]),))
  );

  Ok(())
}
