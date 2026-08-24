//! SPARK 宿主：沙箱加载满足 `plugin-world` 契约的 WASM 组件并调用插件接口。
//!
//! 契约见 `wit/runtime.wit`。插件导出 `spark:runtime/plugin`，不依赖宿主任何能力；
//! 宿主每次调用新建独立 Engine —— 插件 trap 被捕获为 `Err`，不污染宿主。

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};

wasmtime::component::bindgen!({
    path: "../wit/runtime.wit",
    world: "plugin-world",
});

/// 加载组件，调用一次 `name()` 与 `transform(input)`。
/// 插件 panic → trap → `Err`（沙箱隔离，宿主存活）。
pub fn run_plugin(wasm_path: &str, input: &str) -> Result<(String, String)> {
    let engine = Engine::default();
    let component = Component::from_file(&engine, wasm_path)?;
    let linker = Linker::new(&engine);
    // plugin-world 无 import：无需注册宿主函数。
    let mut store = Store::new(&engine, ());
    let instance = PluginWorld::instantiate(&mut store, &component, &linker)?;
    let plugin = instance.spark_runtime_plugin();
    let name = plugin.call_name(&mut store)?;
    let out = plugin.call_transform(&mut store, input)?;
    Ok((name, out))
}
