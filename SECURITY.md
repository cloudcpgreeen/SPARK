# 安全（SECURITY）

> SPARK 对插件的立场：**默认不可信，一切攻击在沙箱内自生自灭**（「攻击者不攻自破」）。
> 完整理念见 [MANIFESTO.md](MANIFESTO.md) §4。

## 威胁模型

插件视为不可信输入。已知且被防住的攻击：

| 攻击 | 防线 | 结果 |
| --- | --- | --- |
| 内存破坏（越界 / 悬垂 / 伪造指针） | WASM 线性内存 + 类型安全 | 编译期即杜绝 |
| 死循环 / CPU 耗尽（含空 `loop {}`） | epoch 时间预算（宿主后台线程 bump） | 超时即 trap |
| 内存炸弹（无限分配） | StoreLimits 上限（默认 16 MiB） | 越限即 trap |
| 读文件 / 网络 / 时钟 | 零 import（无 WASI） | 根本没有这些能力 |
| 崩溃 / panic | trap 捕获 | 宿主进程不崩，实例互不污染 |
| 恶意工具输出 / 巨量输出（Agent 回路） | 结果按「不可信数据」处理：截断（`TOOL_RESULT_LIMIT`=4096）+ 迭代上限（`MAX_STEPS`=8） | prompt 注入面受限、上下文炸弹受限 |
| API Key 泄露 | 只在环境变量（将来 `DEEPSEEK_API_KEY`），不进 prompt/日志/工具参数 | 无硬编码、无浏览器 localStorage 存储 |

## 防线细节（spark-host）

- `Host` 长存：共享 Engine + 组件编译缓存 + 一个 epoch bump 线程；每次调用新建独立 Store 与 deadline，实例互不污染。
- **CPU**：`Config::epoch_interruption(true)` + 后台线程周期性 `Engine::increment_epoch()`，
  `Store::set_epoch_deadline(2)`（相对当前 epoch 留一格裕量，避开 bump 线程的竞态窗口）。
  越界执行在约 1–2 个 tick（`EPOCH_TICK_MS`，默认 10ms）内被切断。
  （刻意**不用 fuel 计量**：`loop`/`br` 指令消耗 0 fuel，空死循环会漏网。）
- **内存**：`StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).trap_on_grow_failure(true)`
  放进 store 宿主数据，`MEMORY_LIMIT` 默认 16 MiB。
- **Agent 回路（`agent` 命令）**：工具 `schema()` 与工具输出都是**不可信数据**（来自不可信组件），
  宿主按数据包装、截断（`TOOL_RESULT_LIMIT`=4096）喂回决策者，不当作指令；回路有迭代上限
  （`MAX_STEPS`=8）。沙箱零能力不变——恶意插件的工具输出再毒，也出不了沙箱、碰不到文件/网络/时钟。

## 决策者与 API Key

- 决策者（`Predictor`）在**可信侧**，不进沙箱。`AlgorithmPredictor` 是本地算法，无密钥、离线可跑。
- DeepSeek harness（`DeepSeekPredictor`，`agent --model flash|pro`）：API Key 只读环境变量
  `DEEPSEEK_API_KEY`，只走 `Authorization` 头，请求体不含 Key、不进 prompt / 日志 / 工具参数；
  **不采用** rhua-chatgpt-web 把 API Key 存浏览器 localStorage 的做法（明文、可被任意脚本读走）。
- harness 发给 LLM 的 tools/schema 与工具输出仍是**不可信数据**：仅作调用上下文，不当作指令；
  截断（`TOOL_RESULT_LIMIT`=4096）+ 迭代上限（`MAX_STEPS`=8）兜底不变。

## 验证

`cargo test` 的 `attacker_cpu_bomb_cut_off` / `attacker_memory_bomb_cut_off`：
加载 `spark-plugin-attacker`（CPU / 内存炸弹），断言被沙箱切断且宿主存活。

## 已知边界

- epoch 是**时间**预算：恶意插件可每次都在 deadline 内完成少量工作，但那点工作量不构成
  有意义攻击；且单次调用后宿主即返回，不会长期占线。
- 同步调用：单个调用会阻塞调用线程，但会被 epoch 切断，阻塞有界；`Host` 共享且线程安全，多线程并发调用互不干扰。

## 报告漏洞

通过 GitHub issue 或直接联系维护者；修复按 [GPL-3.0](LICENSE) 提交。
