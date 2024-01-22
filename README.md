## Calx VM

> Calcit runner can be slow being a dynamic language. Calx VM trying to provide some helper tools for faster computation of very simple but repeated tasks. Ideally Calcit should use WASM for CPU heavy computations.

### Usages

```bash
cargo install calx-vm
calx hello.cirru
```

it starts with a `main` function:

```cirru
fn main ()
  const 1
  call demo
  const 0
  call demo

fn demo (($a i64) ->)
  local.get $a

  if (->)
    do
      const 11
      echo
    do
      const 20
      echo
  const 3
  echo
```

`-s` to show instructions:

```bash
$ calx demos/if.cirru -s
[calx] start preprocessing
loaded fn: CalxFunc main (-> )
  00 Const(I64(1))
  01 Call("demo")
  02 Const(I64(0))
  03 Call("demo")

loaded fn: CalxFunc demo (I64 -> )
  local_names: 0_$a .
  00 LocalGet(0)
  01 JmpIf(5)
  02 Const(I64(20))
  03 Echo
  04 Jmp(8)
  05 Const(I64(11))
  06 Echo
  07 Jmp(8)
  08 Const(I64(3))
  09 Echo

[calx] start running
11
3
20
3
[calx] took 67.250µs: Nil
```

`--emit-binary filename` for encode functions into a binary file.

`--eval-binary` for reading a binary input file to run.

### Syntax Sugar

Code of:

```cirru
fn main ()
  i.add
    const 1
    i.mul
      const 2
      const 3

  echo
    dup

  assert "|expected 7"
    i.eq
      const 7
```

is desugared to:

```cirru
fn main ()
  const 2
  const 3
  i.mul
  const 1
  i.add

  dup
  echo

  const 7
  i.eq
  assert "|expected 7"
```

### Instructions

Find docs on https://docs.rs/calx_vm/ .

Highly inspired by:

- WASM https://github.com/WebAssembly/design/blob/main/Semantics.md
- Lox https://github.com/Darksecond/lox/blob/master/lox-vm/src/bettervm/vm.rs

### Preprocess

Before Calx running the instructions, Calx performs preprocessing to them. There are several tasks:

- `block` and `loop` are expanded since there are `block-end` instructions
- `br` and `br-if` also expanded to `jmp` and `jmp-if` instructions, internally
- stack size is checked to ensure it's consistent among branches, and tidied up at function end
- local variables are renamed to indexes

The codebase would be updated as I'm learning more about WASM.

### License

MIT
