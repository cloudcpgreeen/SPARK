# 技术参考（REFERENCE）

> 给所有说明补一层技术术语：WIT 契约原文、宿主公开 API、沙箱机制与参数、错误码全集、CLI 参考、组件清单。
> 面向严谨读者与机器。友好的版本见 [ARCHITECTURE](ARCHITECTURE.md) 与 [EXAMPLES](EXAMPLES.md)。

## 1. WIT 契约（原文，wit/）

### 1.1 `spark:runtime@0.4.0`（wit/runtime.wit）

```
package spark:runtime@0.4.0;

interface plugin {
  record plugin-info { name: string, version: string, description: string }
  record plugin-error { code: string, message: string }
  record tool-parameter { name: string, parameter-type: string, description: string }
  record tool-schema { name: string, description: string, parameters: list<tool-parameter> }
  info: func() -> plugin-info;
  transform: func(input: string) -> result<string, plugin-error>;
  schema: func() -> list<tool-schema>;
  invoke: func(tool: string, args: string) -> result<string, plugin-error>;
}

world plugin-world { export plugin; }
```

特性：`plugin-world` **零 import**（不含 WASI）。宿主 `bindgen!({ path, world })` 钉死此版本；加载不匹配组件时 `instantiate` 直接失败。`schema()`/`invoke()` 是 **Agent 调用面**：`schema` 返回组件对 LLM 暴露的工具清单（名/描述/参数），`invoke(tool, args)` 按工具名调用，`args` 为 JSON 对象字符串（对应所选 tool-schema 的参数）。`transform` 保留给流水线/直接路径（向后兼容）。字段名用 `parameter-type` 而非 `type`（WIT 保留字）。

### 1.2 `spark:core@0.1.0`（wit/core.wit）

```
package spark:core@0.1.0;

interface health {
  ping: func() -> string;   // 返回 "SPARK <version>"，骨架
}
```

## 2. 宿主公开 API（spark-host）

类型（bindgen 生成，路径 `crate::exports::spark::runtime::plugin::{PluginInfo, PluginError}`；生成类型**不实现 `PartialEq`**）。

`Host` 长存结构：共享 `Engine` + `Mutex<HashMap<PathBuf, Arc<Component>>>` 编译缓存 + 单 epoch bump 线程（`Drop` 停止并 join）。方法取 `&self`，线程安全。

| 签名 | 语义 |
| --- | --- |
| `Host::new() -> anyhow::Result<Host>` | 新建 Engine（`epoch_interruption(true)`）+ 启动 bump 线程（`EPOCH_TICK_MS`=10ms 间隔） |
| `Host::run(&self, wasm_path: &str, input: &str) -> Result<(PluginInfo, Result<String, PluginError>)>` | 沙箱内调 `info()` + `transform(input)`。外层 `Err` = 加载失败 / 契约不匹配 / trap / 资源越限；内层 `Err(PluginError)` = 插件声明式失败（值） |
| `Host::info(&self, wasm_path: &str) -> Result<PluginInfo>` | 仅沙箱内读 `info()`，注册/发现用，不调领域逻辑 |
| `Host::discover(&self, dir: &str) -> Vec<(String, PluginInfo)>` | 扫描目录 `.wasm` 组件（文件名排序），逐个沙箱读 `info()`；读取失败跳过并在 stderr 提示 |
| `Host::pipe(&self, dir: &str, input: &str, names: &[&str]) -> Result<String, PipeFailure>` | 输出串联；`PipeFailure::Declined{step, error}` 声明式失败，`PipeFailure::Trap{step, detail}` trap（`detail` 含 wasm backtrace） |
| `Host::schemas(&self, dir: &str) -> Vec<(String, PluginInfo, Vec<ToolSchema>)>` | Agent 调用面：发现目录下所有插件的工具清单 `(文件名, 插件信息, schema)`，沙箱内读 `schema()`；失败跳过并提示 |
| `Host::invoke(&self, wasm_path: &str, tool: &str, args_json: &str) -> Result<(PluginInfo, Result<String, PluginError>)>` | Agent 调用面：沙箱内调 `info()` + `invoke(tool, args_json)`，隔离与资源上限同 `run` |

### 2.1 Agent 回路（`spark_host::agent`）

| 项 | 语义 |
| --- | --- |
| `trait Predictor` | Agent 回路的决策者：`decide(prompt, history, tools) -> Decision`。两个实现共用一个回路，换决策者宿主零改动 |
| `Decision::Final(String)` / `Decision::Call { tool, args }` | 结束给答复 / 调用某工具（args 为 JSON 对象字符串） |
| `AlgorithmPredictor` | 本地算法预测（离线）：关键词 → 工具选择 + 参数提取（"身份证"→idcard、"银行卡号"→luhn、"人民币/金额"→rmb、"大写"→upper、"倒序"→reverse、"attacker"→attacker；"然后/再" 后按意图词二次调用——"倒序"→reverse、"转大写"→upper——验证回路迭代） |
| `DeepSeekPredictor`（`spark_host::deepseek`） | DeepSeek harness：OpenAI 兼容 Chat Completions（模型 `deepseek-v4-flash`/`deepseek-v4-pro`，端点 `https://api.deepseek.com`），工具 `schema()` 映射成 function calling 让 LLM 决定下一步；Key 只读 `DEEPSEEK_API_KEY`、只走 `Authorization` 头 |
| `run_agent(host, dir, prompt, predictor, max_steps) -> AgentResult` | 回路：发现工具 → Predictor 决策 → `Host::invoke` 沙箱调用 → 结果按数据喂回 → Final 或到 `MAX_STEPS` |
| `AgentResult { answer, calls }` | 最终答复 + 全部工具调用记录 |
| `TOOL_RESULT_LIMIT = 4096` | 工具输出截断上限（防输出炸弹/注入面） |
| `MAX_STEPS = 8` | 回路迭代上限（失控 Predictor 不无限循环） |

## 3. 沙箱机制与参数

| 参数 | 值 | 说明 |
| --- | --- | --- |
| `EPOCH_TICK_MS` | 10 | bump 线程递增 epoch 的间隔（毫秒） |
| `MEMORY_LIMIT` | 16 MiB（16777216 字节） | StoreLimits 线性内存上限 |
| epoch deadline | `当前 epoch + 2` | 越界执行约 10–20ms 内 trap；`+1` 与 bump 线程存在竞态会误伤正常调用 |
| `TOOL_RESULT_LIMIT` | 4096 字节 | Agent 回路里工具输出截断上限（Agent 层，非沙箱） |
| `MAX_STEPS` | 8 | Agent 回路迭代上限（Agent 层，非沙箱） |

- **CPU（epoch 时间预算）**：`Config::epoch_interruption(true)` + 后台线程 `Engine::increment_epoch()` + `Store::set_epoch_deadline(2)`。**不用 fuel**：`loop`/`br` 消耗 0 fuel，空死循环会漏网。
- **内存（StoreLimits）**：`StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).trap_on_grow_failure(true)`，经 `store.limiter(|data| &mut data.limits)` 注册进 Store 宿主数据。
- **隔离**：每次 `run`/`info`/`discover` 新建独立 `Store`；`Engine` 与组件编译缓存跨调用共享。
- **攻击面**：插件零 import，无文件 / 网络 / 时钟能力。

## 4. 插件契约与构建

- 每个插件 = 独立 workspace；`Cargo.toml` 关键字段：`crate-type = ["cdylib"]`、`wit-bindgen-rt = "0.41"`、`[package.metadata.component] package = "spark:runtime"`。
- 构建目标**钉死 `wasm32-unknown-unknown`**（`.cargo/config.toml`）→ 产物零 WASI import（wasip1/wasip2 会把 `wasi:cli/io` 拖进组件）。
- 实现 `Guest`：`info()` + `transform()` + `schema()` + `invoke()`；导出宏必须**两参**：`bindings::export!(Upper with_types_in bindings)`。
- `schema()` 返回工具清单（每个插件至少暴露一个工具，供 Agent 回路/LLM 调用）；`invoke(tool, args_json)` 解析 JSON 参数后复用 `transform` 逻辑（参数缺失/非法 JSON 返回 `args` 错误码）。
- 依赖 `wit-bindgen-rt` + `serde_json`（后者是**编译期库，非宿主 import**——运行时组件仍零 import，沙箱零能力不变）。
- 产物路径：`target/wasm32-unknown-unknown/release/<name>.wasm`（name = 目录名连字符转下划线）。
- 注册：`.wasm` 放进 `plugins/` 即注册；`info().name` 为注册名，宿主零配置。

## 5. 错误码全集

| 来源 | 场景 | code | message |
| --- | --- | --- | --- |
| upper | 输入以 `err` 开头 | `rejected` | `拒绝处理: <input>` |
| idcard | 长度 ≠ 18 | `length` | `长度错误：应为 18 位` |
| idcard | 前 17 位含非数字 | `format` | `格式错误：前 17 位须为数字` |
| idcard | 出生日期非法 | `date` | `出生日期非法` |
| idcard | 校验位不符 | `checksum` | `校验位错误` |
| luhn | 长度不在 13–19 | `length` | `长度错误：应为 13–19 位` |
| luhn | 含非数字字符 | `format` | `格式错误：只能包含数字` |
| luhn | Luhn 校验失败 | `checksum` | `校验位错误` |
| 任意插件 `invoke` | 工具名不匹配 | `tool` | `未知工具: <tool>` |
| 任意插件 `invoke` | args 非 JSON / 缺参数 | `args` | `参数必须是 JSON 对象` / `缺少 <name> 参数` |
| pipe（宿主） | 目录中未找到该 name | `not-found` | `` 未找到插件 `<name>` `` |

**trap 语义**：插件 panic → wasm `unreachable` 指令 → 宿主返回 trap 错误（detail 含 wasm backtrace），宿主进程不崩、实例互不污染。

## 6. CLI 参考

```
spark-host <plugin.wasm> <input>        # 直接给组件路径运行
spark-host run <name> <input>           # 按 info().name 运行（从 plugins/ 发现）
spark-host pipe <input> <name>...       # 流水线：输出串联，fail-fast 定位
spark-host list                         # 发现并列出 plugins/ 下的组件
spark-host agent "<prompt>" [--model flash|pro]  # Agent 回路：本地算法预测决策 + 沙箱工具调用
```

| 输出态 | 格式 | 退出码 |
| --- | --- | --- |
| 成功 | `output: <s>` | 0 |
| 插件声明式失败 | `plugin error [<code>]: <message>` | 0 |
| trap | `plugin trap: <detail>` | 0 |
| pipe 声明式失败 | `✗ 未通过 <step> [<code>]: <message>` | 1 |
| pipe trap | `✗ <step> 崩溃被沙箱捕获: <detail>` | 1 |
| 用法错误 | usage 提示 | 2 |

> 注：`run` 对声明式失败与 trap 均按「可恢复」返回 0；`pipe` 对失败返回 1（fail-fast 需区分）。
>
> `agent`：工具调用轨迹打 stderr（`  → 工具 <name> …`），最终答复打 stdout；不带 `--model` 走离线本地算法，`--model flash|pro` 走 DeepSeek harness（需 `DEEPSEEK_API_KEY` 环境变量，Key 只进 `Authorization` 头，不进 prompt/日志）。

## 7. 组件产物清单（plugins/）

| 目录 | name | version | description | 行为要点 |
| --- | --- | --- | --- | --- |
| spark-plugin | upper | 0.2.0 | 输入转大写 | `trap`/`err` 前缀触发 panic/声明式失败 |
| spark-plugin-reverse | reverse | 0.2.0 | 输入倒序 | `chars().rev().collect()` |
| spark-plugin-attacker | attacker | 0.2.0 | 恶意示例：CPU/内存炸弹 | `loop`/`alloc` 触发炸弹，被沙箱切断 |
| spark-plugin-idcard | idcard | 0.2.0 | 中国身份证号校验（18 位） | 性别/生日/地区 + 4 错误码 |
| spark-plugin-luhn | luhn | 0.2.0 | 银行卡号 Luhn 校验 + 卡组织 | Visa/Mastercard/Amex/UnionPay/未知 + 3 错误码 |
| spark-plugin-rmb | rmb | 0.2.0 | 人民币金额转大写（财会大写） | 零/整 规则、万亿以下、精确到分 |

每个插件都暴露一个同名工具（`schema()`），供 Agent 回路调用。

## 8. 测试矩阵

33 个测试（`cargo test --workspace`）：spark-core 单测 1、并发 1（8 线程共享 Host）、idcard 2（有效/错误码）、isolation 7（happy/声明式/trap/隔离/多插件/CPU 炸弹/内存炸弹）、luhn 2、rmb 2（金额转大写/错误码）、pipe 3（串联/声明式 fail-fast/trap fail-fast）、registry 2（注册/按名运行）、**agent 7（upper / idcard / 两步迭代 / 意图链 upper→reverse / attacker 经 agent 仍被沙箱切断 / 工具输出截断 / rmb）**、**deepseek 单测 6（消息组装 / tools 映射 / 响应解析 tool_calls / 响应解析 Final / 模型映射 / 本地 mock DeepSeek 全链路 e2e——请求格式·Key 仅 Authorization 头·沙箱调用·结果喂回·最终答复）**。组件未构建时自动跳过。
