# 贡献指南（CONTRIBUTING）

> 欢迎你来。SPARK 的规矩不多，但每一条都认真：**契约优先，最小可用，测试说话。**

## 1. 先读这些

| 文档 | 为什么读 |
| --- | --- |
| [MANIFESTO.md](MANIFESTO.md) | 圣经：项目为什么存在、边界在哪。与它冲突的改动，先改它 |
| [CONTRACT.md](CONTRACT.md) | 契约即 WIT：接口是唯一权威来源，跨组件只走契约 |
| [DEVELOPMENT.md](DEVELOPMENT.md) | 怎么写、怎么测、怎么提交（构建与测试命令全在这） |
| [SECURITY.md](SECURITY.md) | 威胁模型：什么能进沙箱、什么能碰宿主 |

## 2. 环境准备

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-component --version 0.21.1 --locked
```

构建与测试命令见 [DEVELOPMENT.md §2](DEVELOPMENT.md)（`cargo test --workspace` + 逐个 `cargo component build --release`）。

## 3. 提 PR 前

1. **契约优先**：改了接口，先改 `wit/` 里的 WIT 并带版本号，再改实现（[CONTRACT.md](CONTRACT.md)）。
2. **测试全绿**：`cargo test --workspace` 通过，插件集成测试无 skip（组件要先构建）。
3. **风格干净**：`cargo fmt --check` 无 diff、`cargo clippy --workspace --all-targets` 无告警。
4. **最小可用**：不加未请求的抽象；非平凡逻辑留一个 runnable check（见 DEVELOPMENT §3、§5）。
5. **提交消息**用 conventional：`feat:` / `fix:` / `refactor:` / `docs:` / `test:` …

## 4. 新增一个插件

1. 复制 `spark-plugin/` 结构（独立 workspace、`crate-type=["cdylib"]`、`wit-bindgen-rt`、`.cargo/config.toml` 钉死 `wasm32-unknown-unknown`）。
2. 实现 `info()` + `transform(input)`，导出用两参宏 `bindings::export!(Ty with_types_in bindings)`。
3. `cargo component build --release` → 把 `.wasm` 放进 `plugins/` 即注册（name 来自组件自身 `info()`，宿主零改动）。
4. 在 `spark-host/tests/` 留一个契约验收测试。

## 5. 报告问题

- **安全漏洞**：走 [SECURITY.md](SECURITY.md) 的上报流程，**不要**开公开 issue。
- 其余 bug / 特性：开 issue，说明复现输入与期望行为。

## 6. 行为准则

请遵守 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。一句话：从爱出发，对写作者温柔，对攻击者不温柔。
