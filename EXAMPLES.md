# 使用范例（EXAMPLES）

> 手把手：一个插件 + 一个宿主，从零跑到结果。目标比 wasmtime 官方示例更容易看懂——不走弯路，只跑通一条路。

## 0. 先建立一张图

SPARK 就两样东西：

- **插件**：一个 `.wasm` 文件，里面几个函数——`info()` 自我介绍、`transform()` 干活、`schema()` 向 Agent/LLM 亮出工具、`invoke()` 按工具名被调用。插件不知道宿主是谁，摸不到文件/网络/时钟。
- **宿主**（spark-host）：一个加载器。拿到 `.wasm`，塞进沙箱，调这些函数，把结果还给你。插件崩溃、死循环、内存炸弹，都只是宿主的一次错误，不会搞死宿主。

整个流程：`写插件 → 构建成 .wasm → 丢给宿主 → 宿主在沙箱里调用 → 拿结果`。

## 1. 最小插件长什么样

`spark-plugin/src/lib.rs`（Upper：输入转大写）就是最小的完整插件：

```rust
struct Upper;

impl Guest for Upper {
    fn info() -> PluginInfo {
        // ① 自我介绍：宿主靠它发现你、认出你
        PluginInfo { name: "upper".into(), version: env!("CARGO_PKG_VERSION").into(), description: "输入转大写".into() }
    }

    fn transform(input: String) -> Result<String, PluginError> {
        // ② 干活：输入 → 输出
        if input.starts_with("trap") {
            panic!("trap requested for input: {input}");   // panic → 宿主捕获为 trap，宿主不崩
        }
        if input.starts_with("err") {
            return Err(PluginError {                          // ③ 声明式失败（值，不是崩溃）
                code: "rejected".into(),
                message: format!("拒绝处理: {input}"),
            });
        }
        Ok(input.to_uppercase())
    }

    fn schema() -> Vec<ToolSchema> {
        // ④ 对 LLM 亮出工具：名字 + 描述 + 参数。Agent 回路/LLM 靠这个决定何时调用你
        vec![ToolSchema {
            name: "upper".into(),
            description: "把输入文本转成大写".into(),
            parameters: vec![ToolParameter {
                name: "text".into(),
                parameter_type: "string".into(),
                description: "要转大写的文本".into(),
            }],
        }]
    }

    fn invoke(tool: String, args: String) -> Result<String, PluginError> {
        // ⑤ 按工具名被调用：args 是 JSON 对象字符串，解析后复用干活逻辑
        let v: serde_json::Value = serde_json::from_str(&args)
            .map_err(|_| PluginError { code: "args".into(), message: "参数必须是 JSON 对象".into() })?;
        let text = v.get("text").and_then(serde_json::Value::as_str)
            .ok_or_else(|| PluginError { code: "args".into(), message: "缺少 text 参数".into() })?;
        Self::transform(text.to_string())
    }
}

bindings::export!(Upper with_types_in bindings);   // ⑥ 声明：我是插件
```

就 6 件事：**自我介绍、干活、两种失败方式、亮出工具、按名被调用、声明自己是插件**。`PluginInfo`/`PluginError`/`ToolSchema`/`ToolParameter`/`Guest` 都是契约 `wit/runtime.wit` 自动生成的，你只填字段、写逻辑。`schema`/`invoke` 只需一行思维量：你的插件在 Agent/LLM 眼里是什么「工具」。

## 2. 构建成 `.wasm`

```bash
cd spark-plugin && cargo component build --release
```

产物：`spark-plugin/target/wasm32-unknown-unknown/release/spark_plugin.wasm`（一个组件文件，零依赖）。

## 3. 丢给宿主跑

**方式 A：直接给路径**（只想快速看结果）

```bash
cargo run -p spark-host -- plugins/spark_plugin.wasm hello
# plugin: upper 0.2.0 — 输入转大写
# output: HELLO
```

**方式 B：放进 `plugins/` 自注册**（插件自己报名字，宿主零配置文件）

```bash
cp spark-plugin/target/wasm32-unknown-unknown/release/spark_plugin.wasm plugins/
cargo run -p spark-host -- list           # 宿主逐个沙箱读取，列出来
cargo run -p spark-host -- run upper hi   # 按名字调用
```

**方式 C：串成流水线**（前一个的输出喂给下一个）

```bash
cargo run -p spark-host -- pipe hello upper reverse
# output: OLLEH   （hello → upper 转大写 HELLO → reverse 倒序 OLLEH）
```

## 4. 想在 Rust 里直接用宿主库？

spark-host 是「库 + CLI」二合一。20 行就能加载插件、调用、拿结果：

```rust
use spark_host::Host;

fn main() {
    let host = Host::new().unwrap();   // 建宿主（沙箱 + 资源上限 + 编译缓存）
    let (info, out) = host.run("plugins/spark_plugin.wasm", "hello").unwrap();
    println!("{} v{} — {}", info.name, info.version, info.description);
    match out {
        Ok(s) => println!("输出: {s}"),
        Err(e) => println!("声明式失败 [{code}]: {message}", code = e.code, message = e.message),
    }
}
```

`Host::run` 的返回分层：`Ok((插件信息, Ok(输出)))` = 成功；`Ok((插件信息, Err(声明式错误)))` = 插件拒绝；只有宿主/沙箱层面的问题（加载失败、trap、资源越限）才是外层 `Err`。

## 5. 写你自己的插件

复制 `spark-plugin` 整个目录，改几处：

1. `Cargo.toml` 的 `package.name`（如 `my-plugin`）
2. `lib.rs` 里 `info()` 的 name、`transform()` 的逻辑，以及 `schema()` 的工具名/描述/参数
3. 重新 `cargo component build --release`

产物文件名 = 目录名连字符转下划线 + `.wasm`（`my-plugin` → `my_plugin.wasm`），放进 `plugins/` 即注册。**宿主零改动**——新增一个插件从来不需要动宿主，这是 SPARK 的硬道理。

## 6. 跑一遍 Agent 回路

插件对你暴露成「工具」。`agent` 命令让决策者决定调哪个工具、然后沙箱里执行——默认是本地算法预测（离线）；加 `--model flash|pro` 换成真实 DeepSeek harness（需 `DEEPSEEK_API_KEY`）：

```bash
cargo run -p spark-host -- agent "把 hello 转大写"                 # → HELLO
cargo run -p spark-host -- agent "校验身份证 110101199001010015"   # → 男 · 1990-01-01 · 地区 110101
cargo run -p spark-host -- agent "校验身份证 110101199001010023 然后倒序"  # 两步：idcard → reverse
cargo run -p spark-host -- agent "把 12345.67 转成人民币大写"        # → 壹万贰仟叁佰肆拾伍元陆角柒分
```

默认全程无网络、不需要任何 API Key——本地算法预测就能把「决策 → 沙箱调用 → 结果喂回」的回路跑通。要上真 LLM 只需两步：`export DEEPSEEK_API_KEY=sk-...`，然后 `cargo run -p spark-host -- agent "把 hello 转大写" --model flash`（或 `--model pro`）即走 DeepSeek harness，回路零改动。
