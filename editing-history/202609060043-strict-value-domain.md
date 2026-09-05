# Strict value domain / strict 值域收口

## 中文

- #59/#61：strict 检查从声明边界扩展到常量和 block/loop/if 签名，包括不可达代码。
- 共用既有 `validate_strict_type`，拒绝无元素类型 List；builder 常量错误在 emission 前原子返回。
- 原实现复现六类失败；新增八组正反回归覆盖直接 Rust IR、parser、builder、host 与 legacy。
- 旧 buffer 错误签名测试改用仍准入的 Str，继续验证 validator 和 binding mismatch；List 提前拒绝单独覆盖。
- 文档记录 unreleased strict 接受范围收紧，修正旧 RFC 的 local 默认值提案说明；精简过时 roadmap 为 #59 → #60。
- 已通过 fmt、完整 debug/release tests、try.sh 全部 demos、严格 all-targets/all-features Clippy 和 cargo package。
- 本提交不发布、不升级 Calcit 依赖、不增加 runtime 分支，不宣称性能提升；下一步按 #60 量化尾调用分配。

## English

- Extend strict admission to constants and control signatures, including dead code, under #59/#61.
- Reuse the existing type rule to exclude unparameterized List and reject builder constants atomically.
- Reproduced six failing categories before the fix; eight regression groups cover Rust IR, parser, builder, hosts, and legacy behavior.
- Keep buffer type-mismatch coverage using admissible Str signatures; cover earlier List rejection separately.
- Document unreleased compatibility tightening and the implemented Uninitialized local behavior; replace the stale roadmap with #59 → #60.
- Verified fmt, complete debug/release tests, all demos, all-targets/all-features Clippy, and cargo package.
- No release, Calcit dependency update, runtime branch, or speedup claim. Tail-call allocation evidence is the next #60 step.
