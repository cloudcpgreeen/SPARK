# 使用范例（EXAMPLES）

> 手把手：一个插件 + 一个宿主，从零跑到结果。目标比 wasmtime 官方示例更容易看懂——不走弯路，只跑通一条路。

## 0. 先建立一张图

SPARK 就两样东西：

- **插件**：一个 `.wasm` 文件，里面就两个函数——`info()` 自我介绍、`transform()` 干活。插件不知道宿主是谁，摸不到文件/网络/时钟。
- **宿主**（spark-host）：一个加载器。拿到 `.wasm`，塞进沙箱，调这两个函数，把结果还给你。插件崩溃、死循环、内存炸弹，都只是宿主的一次错误，不会搞死宿主。

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
}

bindings::export!(Upper with_types_in bindings);   // ④ 声明：我是插件
```

就 4 件事：**自我介绍、干活、两种失败方式、声明自己是插件**。`PluginInfo`/`PluginError`/`Guest` 都是契约 `wit/runtime.wit` 自动生成的，你只填字段、写逻辑。

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

复制 `spark-plugin` 整个目录，改三处：

1. `Cargo.toml` 的 `package.name`（如 `my-plugin`）
2. `lib.rs` 里 `info()` 的 name、`transform()` 的逻辑
3. 重新 `cargo component build --release`

产物文件名 = 目录名连字符转下划线 + `.wasm`（`my-plugin` → `my_plugin.wasm`），放进 `plugins/` 即注册。**宿主零改动**——新增一个插件从来不需要动宿主，这是 SPARK 的硬道理。
