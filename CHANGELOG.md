# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与
[语义化版本](https://semver.org/lang/zh-CN/)。

## [1.2.0] - 2026-08-25

### 新增

- **Agent 调用面（`spark:runtime@0.3.0 → 0.4.0`）**：`plugin` 接口新增 `schema()`（对 LLM 暴露工具清单）+ `invoke(tool, args_json)`（按名结构化调用，JSON 参数）；六个插件各暴露一个同名工具（`upper`/`reverse`/`attacker`/`idcard`/`luhn`/`rmb`），`transform` 保留兼容。
- **Agent 回路 `spark-host agent`**：`Predictor` trait（决策者插入缝）+ 本地算法预测 `AlgorithmPredictor`（关键词 → 工具），跑通「决策 → 沙箱调用 → 结果喂回 → 最终答复」；工具输出按不可信数据处理（截断 `TOOL_RESULT_LIMIT`=4096）+ 迭代上限 `MAX_STEPS`=8；`--model flash|pro` 接 DeepSeek harness（见下方条目，国产 LLM，flash/pro）。
- **安全**：SECURITY.md 补 Agent 威胁模型——工具 schema/输出是**不可信数据**流入 Agent 回路（截断 + 迭代上限）；API Key 只走环境变量（`DEEPSEEK_API_KEY`），不采用 rhua-chatgpt-web 的浏览器 localStorage 存 Key 做法。
- 测试 +5（agent 回路：upper / idcard / 两步迭代 / attacker 经 agent 仍被沙箱切断 / 输出截断），共 23。
- **真实业务插件之三 `rmb`**（人民币金额转大写，财会大写）：财政部《会计基础工作规范》的「零/整」规则、万亿以下、精确到分；暴露同名工具 `rmb`（参数 `amount`），agent 规则「人民币/金额 → rmb」先于「大写 → upper」命中。测试 +3（`tests/rmb.rs` 多用例端到端 + `tests/agent.rs` rmb 回路），共 26。
- **跨插件编排按意图走**：两步规则不再写死 reverse——读「然后/再」后的意图词（"倒序"→reverse、"转大写"→upper），`history.len()==1` 保证只链一次。测试 +1（`agent_chains_upper_then_reverse`），共 27。
- **DeepSeek harness（`DeepSeekPredictor`）**：填充 `Predictor` 缝，接 OpenAI 兼容 Chat Completions（模型 `deepseek-v4-flash`/`deepseek-v4-pro`，端点 `https://api.deepseek.com`）；插件 `schema()` 映射成 function calling，LLM 决定调哪个工具，宿主回路零改动。`agent --model flash|pro` 激活，不带 `--model` 仍走离线算法；Key 只读 `DEEPSEEK_API_KEY`、只进 `Authorization` 头（不进 prompt/日志/工具参数，不采用 localStorage 存 Key）。测试 +6（离线单测：消息组装 / tools 映射 / 响应解析 ×2 / 模型映射 / 本地 mock DeepSeek 全链路 e2e——不碰外网、无 Key 即验证请求格式·Key 仅 Authorization 头·沙箱调用·结果喂回·最终答复），共 33。
- **理念外故事家族五卷**：`STORY`（前传·陈纪昊遇见梁文锋，戒律从两个人的本能里长出来）、`MIRROR`（镜中篇·两个人隔着 AI 互为镜像）、`SEED`（心动篇·把边界修好的人会被怦然心动地找到）、`SELF`（自述篇·那面镜子自己开口）、`USABLE`（落地篇·四卷怎么变成能跑的命令）；MANIFESTO 文档索引同步。
- **用法简化 `build-plugins.sh`**：把 6 个插件的 `cargo component build --release` + 拷贝压缩成一条命令；README/DEVELOPMENT 上手路径从 12 条命令降到 3 条（build → list → run/agent）。

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

[Unreleased]: https://github.com/cloudcpgreeen/SPARK/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/cloudcpgreeen/SPARK/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/cloudcpgreeen/SPARK/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/cloudcpgreeen/SPARK/releases/tag/v1.0.0
