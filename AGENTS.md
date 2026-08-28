# 维护指南 / Maintainer guide

## 中文

- 提交前运行 `cargo fmt --check`、`cargo test` 和仓库 publish workflow 中列出的全部 demos。
- parser ownership 类型变化必须同时验证 crates.io 无 lockfile 构建，避免宽松 semver 范围吸收不兼容版本后使已发布 crate 无法编译。
- `CalxError` 使用可选 boxed VM snapshot；提交前保持 `cargo clippy --all-targets --all-features -- -D warnings` 通过，不要重新扩大公开 `Result` 的 inline error payload。

## English

- Before committing, run `cargo fmt --check`, `cargo test`, and every demo listed by the publish workflow.
- Parser ownership changes must also be checked in a crates.io-style build without relying on a stale lockfile, so a broad semver range cannot break a published crate.
- `CalxError` uses an optional boxed VM snapshot. Keep `cargo clippy --all-targets --all-features -- -D warnings` passing before commits and do not grow the inline error payload of public `Result` APIs again.
