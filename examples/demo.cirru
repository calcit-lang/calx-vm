
fn f1 () (load 1) (echo)


fn main ()
  load 1
  echo
  load 1
  load 2
  add
  echo
  block
    load 1
    load 2
    neg
    add
    block
      load 1
      load 2
      neg
      add
      block
        load 1
        load 2
        neg
        add
  block
    load 1
    load 2
    neg
    add
  quit 0
