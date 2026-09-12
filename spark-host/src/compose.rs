//! P3：Capability 的实现也可以是一个 Component。
//!
//! P2 的实现住在 Host 代码里（`domain_store.rs`）；这里证明**实现可以换成一个组件**：
//!
//! ```text
//! counter-store.wasm  (Consumer, P2 冻结, SHA 85691b8e…)
//!         +                                    wasm-tools compose
//! mem-store.wasm      (Provider, P3 新增)  ────────────────────────>  composed.wasm
//!                                                                     （零 import）
//! ```
//!
//! `composed.wasm` 是 **derived artifact**，不是第三个 Component 源。它的价值在于：
//! import 被组合**消掉**了，所以它能跑在**空 Linker** 上 —— 这正是 P1 的域组件路径。
//!
//! 对照组同样重要：裸 `counter-store.wasm` 在同一个空 Linker 上必须 `Err`，
//! 否则「这条路是空的」就没被证明。
//!
//! **作用域的措辞**：Provider 把状态放在自己的实例内存里（`mem_store.wasm` 的
//! `thread_local!` map），所以组合后能力状态是 instance-scoped。这是**本次实现的后果**，
//! 不是 `spark:capability/storage` 契约或 Component Model 的普遍定律。

use anyhow::Result;
use wasmtime::component::Linker;

use crate::domain_store::StoreWorld;
use crate::{new_store, Host};

// Provider 组件的世界见 `wit/mem-store.wit`：只 export，零 import。
wasmtime::component::bindgen!({
    path: "wit-provider",
    world: "provider-world",
});

/// ① **Provider Component 能实现这个能力** —— 单独实例化 `mem_store.wasm`，
/// `set(key, value)` 之后 `get(key)` 能读回来。
///
/// 只用一个 Provider 实例，所以这里量不到作用域；作用域见 `click_times_selfcontained`。
pub fn storage_roundtrip(
    host: &Host,
    provider_wasm: &str,
    key: &str,
    value: &str,
) -> Result<Option<String>> {
    let component = host.component(provider_wasm)?;
    let linker = Linker::new(&host.engine);
    let mut store = new_store(&host.engine);

    let instance = ProviderWorld::instantiate(&mut store, &component, &linker)?;
    let storage = instance.spark_capability_storage();
    storage.call_set(&mut store, key, value)??;
    Ok(storage.call_get(&mut store, key)??)
}

/// ② **组合消除了 import** —— `composed.wasm` 在**空 Linker** 上实例化、点 `clicks` 次。
///
/// 空 Linker 是这条路的全部证据：只要组件还 import 任何东西，这里就会失败。
/// 同一个函数喂裸 `counter-store.wasm` 必须 `Err`（对照组见 `tests/compose.rs`）。
///
/// `clicks = 0` 就是 ③：**新实例从能力里读到什么**。一次调用 = 一个 Store = 一个 Provider 实例，
/// 所以调用之间读不到彼此 —— 与 P2 的 `domain_store::click_times(..., 0, ...)` 并排对照。
pub fn click_times_selfcontained(host: &Host, wasm_path: &str, clicks: u32) -> Result<u32> {
    let component = host.component(wasm_path)?;
    let linker = Linker::new(&host.engine); // 空：不提供任何 capability 实现
    let mut store = new_store(&host.engine);

    let instance = StoreWorld::instantiate(&mut store, &component, &linker)?;
    let counter = instance.spark_store_counter_store().counter();
    let handle = counter.call_constructor(&mut store)?;
    for _ in 0..clicks {
        counter.call_click(&mut store, handle)?;
    }
    Ok(counter.call_count(&mut store, handle)?)
}
