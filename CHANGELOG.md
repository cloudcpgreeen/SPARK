# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与
[语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [1.1.0] - 2026-08-25

### 新增

- **文档十二篇收全**：REFERENCE（技术参考：WIT 契约原文 / 宿主 API / 沙箱参数 / 错误码全集 / CLI）、CURIOSITY + LOVE（最童趣收尾篇：好奇心够了，但爱是最好的）、MANIFESTO 文档索引补全、README badges（license/CI/tag）+ 社区入口。
- **开源标准全套**：Cargo 元数据（`[workspace.package]` 共享，core/host 继承并补 description/repository/keywords/categories，5 插件补 repository/description）、GitHub Actions CI（fmt + clippy + 单元测试 + 插件集成测试 18 个无 skip）、CONTRIBUTING / CODE_OF_CONDUCT / CHANGELOG、issue/PR 模板、.editorconfig。

### 修复

- spark-core 契约注释引用 `wit/spark.wit` → `wit/core.wit`（实际权威文件，纯文档修正）。
- 修 clippy `doc_lazy_continuation` 告警；`cargo fmt` 归整。

## [1.0.0] - 2026-08-24

### 新增

- **契约即 WIT**：`spark:runtime@0.3.0` `plugin-world`，插件零 import（不含 WASI），宿主 bindgen 钉死契约版本。
- **声明式失败**：`transform(input) -> result<string, plugin-error>`（结构化 `code`/`message`）；panic 捕获为 trap，宿主进程不崩。
- **宿主 `spark-host`（wasmtime 47）**：`Host` 长存结构（共享 Engine + 组件编译缓存 + 单 epoch bump 线程），每次调用新建独立 Store，方法取 `&self` 可多线程并发。
- **沙箱资源有界**：CPU 走 epoch 时间预算（越界约 10–20ms 切断），内存走 StoreLimits（16 MiB 上限）；`attacker` 插件的 CPU/内存炸弹被切断。
- **插件自注册/发现**：`.wasm` 放进 `plugins/` 即注册，name 来自组件 `info()`，宿主零配置；CLI `list` / `run <name>`。
- **流水线 `Host::pipe`**：输出串联，任一步声明式失败或 trap 即 fail-fast 并定位到具体插件。
- **示例插件 5 个**：`upper`（转大写）、`reverse`（倒序）、`attacker`（安全验证）、`idcard`（GB 11643-1999 身份证校验）、`luhn`（银行卡 Luhn + 卡组织识别）。
- **开源全套**：LICENSE（GPL-3.0）、CONTRIBUTING、CODE_OF_CONDUCT、issue/PR 模板、GitHub Actions CI（fmt + clippy + test + 插件集成测试）、Cargo 元数据（workspace.package 共享）。
- **文档十二篇**：MANIFESTO / CONTRACT / DEVELOPMENT / DEPLOYMENT / SECURITY / ARCHITECTURE / EXAMPLES / RELATIONSHIPS / REFERENCE / ESSAY / CURIOSITY / LOVE。

### 修复

- epoch deadline 取 `当前 epoch + 2` 而非 `+1`：消除 bump 线程竞态导致的偶发误 trap。

[Unreleased]: https://github.com/cloudcpgreeen/SPARK/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/cloudcpgreeen/SPARK/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/cloudcpgreeen/SPARK/releases/tag/v1.0.0
