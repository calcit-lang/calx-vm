## Calx VM

> Calcit runner can be slow being a dynamic language. Calx VM trying to provide some helper tools for faster computation of very simple but repeated tasks. Ideally Calcit should use WASM for CPU heavy computations.

### Usages

TODO

### Instructions

Highly inspired by:

- WASM https://github.com/WebAssembly/design/blob/main/Semantics.md
- Lox https://github.com/Darksecond/lox/blob/master/lox-vm/src/bettervm/vm.rs

For binary op, top value puts on right.

| Code              | Usage                                                   | Note        |
| ----------------- | ------------------------------------------------------- | ----------- |
| `local`           | new local variable                                      | (not ready) |
| `local.set $idx`  | set value at `$idx`                                     |             |
| `local.tee $idx`  | set value at `$idx`, and also load it                   |             |
| `local.get $idx`  | get value at `$idx` load on stack                       |             |
| `global.set $idx` | set global value at `$idx`                              |             |
| `global.get $idx` | get global value at `$idx`                              |             |
| `load $v`         | load value `$v` on stack                                |             |
| `dup`             | duplicate top value on stack                            |             |
| `drop`            | drop top value from stack                               |             |
| `i.add`           | add two i64 numbers on stack into one                   |             |
| `i.mul`           | multiply two i64 numbers on stack into one              |             |
| `i.div`           | divide two i64 numbers on stack into one                |             |
| `i.rem`           | calculate reminder two i64 numbers on stack into one    |             |
| `i.neg`           | negate i64 numbers on top of stack                      |             |
| `i.shr $bits`     | call SHR `$bits` bits on i64 numbers on top of stack    |             |
| `i.shl $bits`     | call SHL `$bits` bits on i64 numbers on top of stack    |             |
| `i.eq`            | detects if two i64 numbers on stack equal               |             |
| `i.ne`            | detects if two i64 numbers on stack not equal           |             |
| `i.lt`            | litter than, compares two i64 numbers on stack          |             |
| `i.gt`            | greater than, compares two i64 numbers on stack         |             |
| `i.le`            | litter/equal than, compares two i64 numbers on stack    |             |
| `i.ge`            | greater/equal than, compares two i64 numbers on stack   |             |
| `add`             | add two f64 numbers on stack into one                   |             |
| `mul`             | multiply two f64 numbers on stack into one              |             |
| `div`             | divide two f64 numbers on stack into one                |             |
| `neg`             | negate f64 numbers on top of stack                      |             |
| `list.new`        |                                                         | TODO        |
| `list.get`        |                                                         | TODO        |
| `list.set`        |                                                         | TODO        |
| `link.new`        |                                                         | TODO        |
| `and`             |                                                         | TODO        |
| `or`              |                                                         | TODO        |
| `br $n`           | break `$n` level of block, 0 means end of current block |             |
| `br-if $n`        | like `br $n` but detects top value on stack first       |
| `block`           | declare a block                                         |             |
| `loop`            | declare a loop block                                    |             |
| (BlockEnd)        | internal mark for ending a block                        | Internal    |
| `echo`            | pop value from stack and print                          |             |
| `call $f $size`   | call function `$f` with `$size` params                  |             |
| `unreachable`     | throw unreachable panic                                 |             |
| `nop`             | No op                                                   |             |
| `quit $code`      | quit program and return exit code `$code`               |             |
| `return`          |                                                         | TODO        |

### License

MIT
