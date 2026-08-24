//! SPARK 宿主：沙箱加载满足 `plugin-world` 契约的 WASM 组件并调用插件接口。
//!
//! 契约见 `wit/runtime.wit`（`spark:runtime@0.2.0`）。插件导出 `spark:runtime/plugin`，
//! 不依赖宿主任何能力。宿主 bindgen 钉死契约版本：加载不匹配组件时 instantiate 直接失败。
//! 插件 `transform` 返回 `result<string, string>`：声明式 `err`（值）与 panic（trap）
//! 都是可恢复错误，宿主不崩。
//! 沙箱强制**资源有界**：
//! - CPU 走 epoch 时间预算 —— 后台线程周期性 bump epoch，任何越界执行（含空 `loop {}`，
//!   fuel 计量的已知漏洞）超时即 trap；
//! - 内存走 StoreLimits —— 内存炸弹在越限时被切断。
//! 攻击在预算耗尽时 → `Err`，宿主永远可响应；每次调用新建独立 Engine/Store/后台线程，
//! 插件互不污染。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::exports::spark::runtime::plugin::PluginInfo;
use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};

/// Store 宿主数据：存放资源限制器（`Store::limiter` 的闭包要求返回借自 store 数据的引用）。
struct HostData {
    limits: StoreLimits,
}

wasmtime::component::bindgen!({
    path: "../wit/runtime.wit",
    world: "plugin-world",
});

/// 后台线程 bump epoch 的间隔：插件执行超过约一个 tick 即视为失控，被切断。
pub const EPOCH_TICK_MS: u64 = 10;

/// 插件线性内存上限：内存炸弹在越限时被切断。
pub const MEMORY_LIMIT: usize = 16 * 1024 * 1024; // 16 MiB

/// 加载组件，调用一次 `info()` 与 `transform(input)`。
/// 返回 `(插件信息, 变换结果)`；`transform` 的 `Err` 是插件声明式失败（值，非崩溃），
/// 外层 `Err` 才是宿主/沙箱错误（trap、加载失败、资源越限）。
pub fn run_plugin(
    wasm_path: &str,
    input: &str,
) -> Result<(PluginInfo, std::result::Result<String, String>)> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;

    // 后台线程周期性 bump epoch：越过 deadline 的 wasm 立即 trap（含空死循环）。
    let stop = Arc::new(AtomicBool::new(false));
    let bumper_engine = engine.clone();
    let bumper_stop = stop.clone();
    let bumper = thread::spawn(move || {
        while !bumper_stop.load(Ordering::Relaxed) {
            bumper_engine.increment_epoch();
            thread::sleep(Duration::from_millis(EPOCH_TICK_MS));
        }
    });

    let result = run_plugin_inner(&engine, wasm_path, input);

    stop.store(true, Ordering::Relaxed);
    bumper.join().ok();
    result
}

fn run_plugin_inner(
    engine: &Engine,
    wasm_path: &str,
    input: &str,
) -> Result<(PluginInfo, std::result::Result<String, String>)> {
    let component = Component::from_file(engine, wasm_path)?;
    let linker = Linker::new(engine);
    let host = HostData {
        limits: StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .trap_on_grow_failure(true)
            .build(),
    };
    let mut store = Store::new(engine, host);
    store.limiter(|data| &mut data.limits);
    store.set_epoch_deadline(1); // 当前 epoch +1：bumper 下一次 bump 即触发
    let instance = PluginWorld::instantiate(&mut store, &component, &linker)?;
    let plugin = instance.spark_runtime_plugin();
    let info = plugin.call_info(&mut store)?;
    let out = plugin.call_transform(&mut store, input)?;
    Ok((info, out))
}
