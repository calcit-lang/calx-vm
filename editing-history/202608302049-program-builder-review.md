# ProgramBuilder review：local name 去重复杂度

## 修改

- `FunctionBuilder` 增加专用 `HashSet<Rc<str>>` 追踪 parameter/local 名称。
- 保留 `local_names: Vec<String>` 作为稳定的有序函数 metadata。
- 重复声明检查由逐次线性扫描改为平均 O(1) lookup/insert，避免生成大量 locals 时整体退化为
  O(n²)。

## 验证

- `cargo fmt --check`
- `cargo test --test builder_tests`
- `cargo clippy --all-targets --all-features -- -D warnings`
