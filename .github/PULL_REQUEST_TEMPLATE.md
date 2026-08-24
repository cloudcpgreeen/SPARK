## 摘要

（改动是什么、为什么。契约优先：先说明是否动了 `wit/`。）

## 测试

- [ ] `cargo fmt --check` 无 diff
- [ ] `cargo clippy --workspace --all-targets` 无告警
- [ ] `cargo test --workspace` 全绿
- [ ] 插件组件已 `cargo component build --release` 构建，集成测试无 skip

## 契约

- 是否改了 `wit/*.wit`？
  - 若改了：版本号已按 semver 同步（见 CONTRACT.md），宿主 bindgen 与实现已对齐。
