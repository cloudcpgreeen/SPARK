# SPARK

> 组件运行时：领域逻辑写成满足 `spark:runtime` 契约的 WASM 组件，宿主沙箱加载调用。契约即 WIT。

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![CI](https://github.com/cloudcpgreeen/SPARK/actions/workflows/ci.yml/badge.svg)](https://github.com/cloudcpgreeen/SPARK/actions/workflows/ci.yml)
[![GitHub tag](https://img.shields.io/github/v/tag/cloudcpgreeen/SPARK)](https://github.com/cloudcpgreeen/SPARK/releases)

## 文档

| 文档 | 内容 |
| --- | --- |
| [`MANIFESTO.md`](MANIFESTO.md) | 圣经 · 理念宣言：项目为什么存在、是什么、边界在哪、怎么保护自己 |
| [`CONTRACT.md`](CONTRACT.md) | 约定一 · 契约：接口即契约，契约即 **WIT**；契约优先工作流与版本规则 |
| [`DEVELOPMENT.md`](DEVELOPMENT.md) | 约定二 · 开发：项目结构、构建测试、代码风格、新增功能流程 |
| [`DEPLOYMENT.md`](DEPLOYMENT.md) | 约定三 · 交付：交付门禁、版本发布、运行/配置/安全 |
| [`SECURITY.md`](SECURITY.md) | 安全：威胁模型、上报流程 |
| [`EXAMPLES.md`](EXAMPLES.md) | 使用范例：手把手从零写插件、构建、跑起来 |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | 完整的版本：框架怎么形成的 + 具体实现逻辑 |
| [`RELATIONSHIPS.md`](RELATIONSHIPS.md) | 人物关系图：圣经里的所有人物与关系，一张网收束 |
| [`REFERENCE.md`](REFERENCE.md) | 技术参考：WIT 契约原文 / 宿主 API / 沙箱参数 / 错误码全集 / CLI——全部用技术术语钉死 |
| [`ESSAY.md`](ESSAY.md) | 理念外一篇：论论文安全——不是防学生，是护写作者 |
| [`CURIOSITY.md`](CURIOSITY.md) | 理念外·最童趣篇之一：好奇心就够了（Curiosity is All You Need） |
| [`LOVE.md`](LOVE.md) | 理念外·最童趣篇之二：但爱是最好的（But Love is The Best）——整个故事的钥匙 |

## 社区

- [CONTRIBUTING.md](CONTRIBUTING.md) — 贡献指南（契约优先、提 PR 前检查单）
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — 行为准则
- [CHANGELOG.md](CHANGELOG.md) — 变更日志

## 结构

- Cargo workspace：`spark-core`（无 HTTP 领域库）、`spark-host`（wasmtime 宿主）。
- `spark-plugin`：插件组件（独立 workspace），产出零依赖 WASM 组件，导出 `spark:runtime/plugin`。
- `wit/`：`core.wit`（`spark:core@0.1.0` 骨架）、`runtime.wit`（`spark:runtime@0.4.0`，`plugin-world` 契约：`transform` 返回 `result<string, plugin-error>`、`info` 带元数据，另有 Agent 调用面 `schema`/`invoke`）。

## 快速上手

```bash
./build-plugins.sh                                # 一键构建全部插件并装入 plugins/
cargo run -p spark-host -- list                   # 看都有什么插件
cargo run -p spark-host -- run upper "hello"      # 按名字运行（沙箱内）
```

示例插件 `Upper`：输入转大写；输入以 `trap` 开头时触发 panic，宿主以 trap 捕获、进程不崩（沙箱隔离）；输入以 `err` 开头时返回声明式错误（值，不是崩溃）。

### Agent 回路（决策者 → 沙箱工具调用）

插件对 LLM 暴露为**工具**（`schema()`/`invoke()`）。`agent` 命令两条路：默认**本地算法预测**决策（无需网络、无需 API Key）；加 `--model flash|pro` 走**真实 DeepSeek harness**（需 `DEEPSEEK_API_KEY` 环境变量，Key 只进 `Authorization` 头）：

```bash
cargo run -p spark-host -- agent "把 hello 转大写"                 # upper → HELLO（离线）
cargo run -p spark-host -- agent "校验身份证 110101199001010015"   # idcard → 男 · 1990-01-01 · 地区 110101
cargo run -p spark-host -- agent "校验身份证 110101199001010023 然后倒序"  # 两步：idcard → reverse（回路迭代）
cargo run -p spark-host -- agent "让 attacker 跑 loop"             # 恶意插件仍被沙箱切断，宿主存活
cargo run -p spark-host -- agent "把 12345.67 转成人民币大写"        # rmb → 壹万贰仟叁佰肆拾伍元陆角柒分

export DEEPSEEK_API_KEY=sk-...                                  # 换真 LLM：仅此一步
cargo run -p spark-host -- agent "把 hello 转大写" --model flash    # DeepSeek V4 Flash harness
```

### 插件自注册（注册/发现）

把满足 `plugin-world` 契约的 `.wasm` 组件放进 `plugins/` 目录即注册（name 来自组件自身 `info()`，宿主零配置文件）：

```bash
./build-plugins.sh                          # 一键构建全部 6 个插件并装入 plugins/
cargo run -p spark-host -- list             # 发现并列出
cargo run -p spark-host -- run upper hi     # 按名字运行
cargo run -p spark-host -- pipe hi upper reverse   # 流水线：依次经 upper、reverse 串联
```

## 新增插件

写一个满足 `plugin-world` 契约的组件（见 `wit/runtime.wit`），宿主零改动即可加载。插件实现四个函数：`info()`（自我介绍）、`transform(input)`（流水线/直接路径）、`schema()`（对 LLM 暴露的工具清单）、`invoke(tool, args_json)`（Agent 路径的结构化调用）。

现有示例插件：`spark-plugin`（upper，输入转大写）、`spark-plugin-reverse`（reverse，输入倒序）、`spark-plugin-attacker`（恶意示例，安全验证用：CPU/内存炸弹会被沙箱切断）、`spark-plugin-idcard`（真实业务算法：中国身份证号校验，返回性别/出生日期/地区或结构化错误）、`spark-plugin-luhn`（真实业务算法：银行卡号 Luhn 校验 + 卡组织识别）、`spark-plugin-rmb`（真实业务算法：人民币金额转大写）。六个插件用同一个 `spark-host` 二进制加载，宿主零改动。

`pipe` 把多个插件串成流水线：前一个插件输出喂给下一个，任一步声明式失败或 trap 即 fail-fast 并定位到具体插件（如 `pipe <身份证号> idcard luhn`：身份证校验通过后，Luhn 因长度拒绝并报 `[length]`）。
