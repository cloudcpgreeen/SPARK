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

## 防线细节（spark-host）

- 每次调用 `run_plugin` 新建独立 Engine/Store + epoch bump 线程，实例互不污染。
- **CPU**：`Config::epoch_interruption(true)` + 后台线程周期性 `Engine::increment_epoch()`，
  `Store::set_epoch_deadline(1)`。任何执行超过一个 tick（`EPOCH_TICK_MS`，默认 10ms）的
  wasm 立即被切断。
  （刻意**不用 fuel 计量**：`loop`/`br` 指令消耗 0 fuel，空死循环会漏网。）
- **内存**：`StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).trap_on_grow_failure(true)`
  放进 store 宿主数据，`MEMORY_LIMIT` 默认 16 MiB。

## 验证

`cargo test` 的 `attacker_cpu_bomb_cut_off` / `attacker_memory_bomb_cut_off`：
加载 `spark-plugin-attacker`（CPU / 内存炸弹），断言被沙箱切断且宿主存活。

## 已知边界

- epoch 是**时间**预算：恶意插件可每次都在 deadline 内完成少量工作，但那点工作量不构成
  有意义攻击；且单次调用后宿主即返回，不会长期占线。
- 单线程同步宿主：长时间运行的插件会阻塞调用线程——但会被 epoch 切断，阻塞有界。

## 报告漏洞

通过 GitHub issue 或直接联系维护者；修复按 [GPL-3.0](LICENSE) 提交。
