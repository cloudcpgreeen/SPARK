# SPARK

> 目标待定 —— idea 正在咨询 DeepSeek 的路上。本仓库先立好协作约定与骨架，等 idea 落地后按「契约优先」填进 WIT。

## 三份约定

| 文档 | 内容 |
| --- | --- |
| [`CONTRACT.md`](CONTRACT.md) | 约定一 · 契约：接口即契约，契约即 **WIT**；契约优先工作流与版本规则 |
| [`DEVELOPMENT.md`](DEVELOPMENT.md) | 约定二 · 开发：项目结构、构建测试、代码风格、新增功能流程 |
| [`DEPLOYMENT.md`](DEPLOYMENT.md) | 约定三 · 交付：交付门禁、版本发布、运行/配置/安全（预留） |

## 当前状态

- Cargo workspace：`spark-core`（无 HTTP 领域库，骨架）。
- `wit/spark.wit`：`spark:core@0.1.0` 契约骨架（示范写法，非业务接口）。
- 构建 / 测试：`cargo build`、`cargo test`。

## 下一步

拿到 idea 后，第一步是把它翻译成 `wit/` 下的 WIT 接口（见 CONTRACT.md §3），再实现。
