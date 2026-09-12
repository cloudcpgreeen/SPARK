//! 跨端域组件宿主：`spark:ui@0.1.0` 的 `domain-world`（第二个 world）。
//!
//! 与 `plugin-world` **刻意分开**，因为两者是两套信任模型：
//! - `plugin-world`：零 import 的**不可信插件沙箱**（安全边界，见 `lib.rs`）；
//! - `domain-world`：**前后端统一的域组件**——同一个 `.wasm` 也跑在 Web / RN 前端（jco 适配），
//!   组件不知道自己在哪一端。UI 是宿主的事，状态与行为是组件的事。
//!
//! 复用 [`crate::Host`] 的 Engine、组件编译缓存、epoch bump 线程与 [`crate::new_store`]
//! 的资源上限（内存 16 MiB + epoch 时间预算），因此沙箱安全语义与插件路径一致。

use anyhow::Result;
use wasmtime::component::Linker;

use crate::{new_store, Host};

wasmtime::component::bindgen!({
    path: "../wit/ui.wit",
    world: "domain-world",
});

/// 在沙箱内新建一个 button 实例，点 `clicks` 次，返回最终 count。
///
/// 同一个 `button.wasm` 也可被 Web / RN 宿主加载；各宿主各有自己的实例与状态，
/// 共享的是契约与行为。
pub fn click_times(host: &Host, wasm_path: &str, clicks: u32) -> Result<u32> {
    let component = host.component(wasm_path)?;
    let linker = Linker::new(&host.engine);
    let mut store = new_store(&host.engine);
    let instance = DomainWorld::instantiate(&mut store, &component, &linker)?;
    let counter = instance.spark_ui_button().counter();
    let handle = counter.call_constructor(&mut store)?;
    for _ in 0..clicks {
        counter.call_click(&mut store, handle)?;
    }
    Ok(counter.call_count(&mut store, handle)?)
}
