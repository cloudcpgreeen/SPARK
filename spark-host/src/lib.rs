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
//!
//! 注册/发现：插件**自注册** —— 把满足 `plugin-world` 的 `.wasm` 组件放进目录即被
//! [`discover_plugins`] 发现，`info()` 里的 name 就是注册名，宿主零配置文件。

use std::fs;
use std::path::Path;
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

/// 沙箱外壳：新建独立 Engine/Store + epoch bump 线程跑 `f`，结束后线程退出。
/// 每次调用天然隔离 —— trap / 资源越限不污染后续调用。
fn with_sandbox<T>(f: impl FnOnce(&Engine) -> Result<T>) -> Result<T> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;

    let stop = Arc::new(AtomicBool::new(false));
    let bumper_engine = engine.clone();
    let bumper_stop = stop.clone();
    let bumper = thread::spawn(move || {
        while !bumper_stop.load(Ordering::Relaxed) {
            bumper_engine.increment_epoch();
            thread::sleep(Duration::from_millis(EPOCH_TICK_MS));
        }
    });

    let result = f(&engine);
    stop.store(true, Ordering::Relaxed);
    bumper.join().ok();
    result
}

/// 新建带资源上限的 Store：内存上限 + 越界即 trap + epoch deadline。
fn new_store(engine: &Engine) -> Store<HostData> {
    let host = HostData {
        limits: StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .trap_on_grow_failure(true)
            .build(),
    };
    let mut store = Store::new(engine, host);
    store.limiter(|data| &mut data.limits);
    store.set_epoch_deadline(1); // 当前 epoch +1：bumper 下一次 bump 即触发
    store
}

/// 加载组件，调用一次 `info()` 与 `transform(input)`。
/// 返回 `(插件信息, 变换结果)`；`transform` 的 `Err` 是插件声明式失败（值，非崩溃），
/// 外层 `Err` 才是宿主/沙箱错误（trap、加载失败、资源越限）。
pub fn run_plugin(
    wasm_path: &str,
    input: &str,
) -> Result<(PluginInfo, std::result::Result<String, String>)> {
    with_sandbox(|engine| {
        let component = Component::from_file(engine, wasm_path)?;
        let linker = Linker::new(engine);
        let mut store = new_store(engine);
        let instance = PluginWorld::instantiate(&mut store, &component, &linker)?;
        let plugin = instance.spark_runtime_plugin();
        let info = plugin.call_info(&mut store)?;
        let out = plugin.call_transform(&mut store, input)?;
        Ok((info, out))
    })
}

/// 仅沙箱内读取插件元数据（`info()`）：注册/发现用，不调用领域逻辑。
pub fn plugin_info(wasm_path: &str) -> Result<PluginInfo> {
    with_sandbox(|engine| {
        let component = Component::from_file(engine, wasm_path)?;
        let linker = Linker::new(engine);
        let mut store = new_store(engine);
        let instance = PluginWorld::instantiate(&mut store, &component, &linker)?;
        Ok(instance.spark_runtime_plugin().call_info(&mut store)?)
    })
}

/// 发现 `dir` 下的插件：逐个沙箱读取 `info()`，得到 `(文件名, 插件信息)`。
/// 插件**自注册** —— name 就是注册名；读失败的组件跳过并在 stderr 提示，不中断整批。
pub fn discover_plugins(dir: &str) -> Vec<(String, PluginInfo)> {
    let Ok(entries) = fs::read_dir(dir) else {
        eprintln!("插件目录不存在: {dir}");
        return Vec::new();
    };
    let mut wasm_files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("wasm"))
        .collect();
    wasm_files.sort();

    let mut found = Vec::new();
    for path in wasm_files {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        match plugin_info(&path.to_string_lossy()) {
            Ok(info) => found.push((name, info)),
            Err(e) => eprintln!("跳过 {name}：{e:#}"),
        }
    }
    found
}
