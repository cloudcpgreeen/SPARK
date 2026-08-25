# 开发约定（DEVELOPMENT）

> 约定二 · 怎么写、怎么测、怎么提交。

## 1. 项目结构

Cargo workspace，成员：

```
spark/
├── Cargo.toml        # workspace 定义（spark-core、spark-host）
├── spark-core/       # 无 HTTP 领域库：NAME / contract_version
├── spark-host/       # wasmtime 宿主：沙箱加载 plugin-world 组件并调用
├── spark-plugin/     # 插件组件（独立 workspace，仅 cargo component build）
├── wit/              # WIT 契约：core.wit、runtime.wit（见 CONTRACT.md）
└── CONTRACT.md       # 约定一 · 契约（WIT）
   DEVELOPMENT.md     # 约定二 · 开发（本文件）
   DEPLOYMENT.md      # 约定三 · 交付
```

- 领域逻辑（模型 / 状态流转 / 纯函数）放 `spark-core`。
- 宿主（`spark-host`）与插件（`spark-plugin`）只通过 `wit/runtime.wit` 的 `plugin-world` 契约通信；插件 = 满足契约的零依赖 WASM 组件，宿主沙箱加载。
- WIT 是接口唯一权威来源；接口与实现分离（见 CONTRACT.md）。

## 2. 构建与测试

```bash
cargo build / cargo test                     # 根 workspace（spark-core + spark-host）
./build-plugins.sh                                          # 一键构建全部 6 个插件并装入 plugins/（逐个 cargo component build 的等价物）
cargo run -p spark-host -- spark-plugin/target/wasm32-unknown-unknown/release/spark_plugin.wasm <input>
cargo run -p spark-host -- pipe <input> upper reverse   # 流水线：输出串联，任一步失败即 fail-fast
cargo run -p spark-host -- agent "把 hello 转大写"       # Agent 回路：本地算法预测 + 沙箱工具调用（无网络）
cargo run -p spark-host -- agent "把 hello 转大写" --model flash  # DeepSeek harness（需 DEEPSEEK_API_KEY，Key 只走环境变量）
```

- 插件组件固定 `--target wasm32-unknown-unknown`（`.cargo/config.toml` 已钉死），产物零 WASI import，纯粹导出 `spark:runtime/plugin`。
- `spark-host` 集成测试覆盖 happy path / 声明式失败（`result` 的 err，`code` 可断言）/ trap 捕获 / trap 后隔离 / 多插件可插拔 / 攻击者切断（`tests/isolation.rs`）、插件自注册与按 name 解析运行（`tests/registry.rs`）、共享 `Host` 多线程并发调用且 trap 不串（`tests/concurrency.rs`）、流水线输出串联与 fail-fast 定位（`tests/pipe.rs`）、Agent 回路（决策 → 沙箱调用 → 结果喂回，`tests/agent.rs`）、金额转大写（`tests/rmb.rs`）、DeepSeek harness 离线单测（消息组装 / tools 映射 / 响应解析，`src/deepseek.rs`）；组件未构建时自动跳过（先执行上面的 `cargo component build`）。
- 沙箱资源有界：CPU 走 epoch 时间预算、内存走 StoreLimits（见 [SECURITY.md](SECURITY.md)）。
- 测试不依赖网络/外部服务：harness 的 HTTP 调用只在带 `DEEPSEEK_API_KEY` 手动执行 `agent --model` 时发生，测试套件不打真实网络。

## 3. 代码风格（ponytail）

- **最小可用**：复用优先，不加未请求的抽象；删除优于新增；不造用不到的框架。
- 非平凡逻辑（分支 / 循环 / 解析 / 涉及钱与安全）必须留**一个** runnable check（assert 测试）；平凡一行逻辑不需要。
- 默认不写注释；只在 WHY 非显然处写一句。

## 4. 新增功能流程

1. 先写 WIT 契约（见 CONTRACT.md 契约优先工作流），提交里带版本号。
2. 实现，满足契约。
3. `cargo test` 全绿后提交；提交消息用 conventional（`feat:` / `fix:` / `refactor:` …）。
4. 交付门禁见 DEPLOYMENT.md。

## 5. 测试纪律

- 契约验收测试按接口语义断言（输入 → 期望输出），不测实现细节。
- `spark-core::contract_version()` 必须与 `wit/core.wit` 的 package 版本对齐（当前 `0.1.0`）；宿主/插件契约版本以 `wit/runtime.wit`（`spark:runtime@0.4.0`）为准——契约与实现脱节是 bug。
