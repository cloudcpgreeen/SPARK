# 开发约定（DEVELOPMENT）

> 约定二 · 怎么写、怎么测、怎么提交。

## 1. 项目结构

Cargo workspace，成员：

```
spark/
├── Cargo.toml        # workspace 定义
├── spark-core/       # 无 HTTP 领域库（领域逻辑优先放这里）
├── wit/              # WIT 契约（约定一，见 CONTRACT.md）
└── CONTRACT.md       # 约定一 · 契约（WIT）
   DEVELOPMENT.md     # 约定二 · 开发（本文件）
   DEPLOYMENT.md      # 约定三 · 交付
```

- 领域逻辑（模型 / 状态流转 / 纯函数）放 `spark-core`。
- 未来的 HTTP/CLI/边界层 crate 只做路由与适配，不写领域逻辑。
- WIT 是接口唯一权威来源；接口与实现分离（见 CONTRACT.md）。

## 2. 构建与测试

```bash
cargo build        # 根 workspace 构建
cargo test         # 根 workspace 测试
cargo test -p spark-core
```

- WIT 构建工具（`cargo-component` / `wasm-tools`）在**第一个真实接口落地**时接入；当前骨架不引入。
- 测试不依赖网络/外部服务（纯库友好，`spark-core` 无 HTTP）。

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
- `spark-core::contract_version()` 必须与 `wit/spark.wit` 的 package 版本对齐（当前 `0.1.0`）——契约与实现脱节是 bug。
