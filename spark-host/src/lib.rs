//! SPARK 宿主：沙箱加载满足 `plugin-world` 契约的 WASM 组件并调用插件接口。
//!
//! 契约见 `wit/runtime.wit`（`spark:runtime@0.3.0`）。插件导出 `spark:runtime/plugin`，
//! 不依赖宿主任何能力。宿主 bindgen 钉死契约版本：加载不匹配组件时 instantiate 直接失败。
//! 插件 `transform` 返回 `result<string, plugin-error>`（`code`/`message` 结构化错误）：
//! 声明式 `err`（值，可按 code 分支）与 panic（trap）都是可恢复错误，宿主不崩。
//!
//! **宿主 = [`Host`]**：长存共享 Engine + 组件编译缓存 + 一个 epoch bump 线程。
//! 每次 `run`/`info` 新建独立 Store（沙箱隔离不变，实例互不污染），`Host` 方法取 `&self`，
//! 可被多线程并发调用。
//! 沙箱强制**资源有界**：
//! - CPU 走 epoch 时间预算 —— 后台线程周期性 bump epoch，任何越界执行（含空 `loop {}`，
//!   fuel 计量的已知漏洞）超时即 trap；
//! - 内存走 StoreLimits —— 内存炸弹在越限时被切断。
//!
//! 攻击在预算耗尽时 → `Err`，宿主永远可响应。
//!
//! 注册/发现：插件**自注册** —— 把满足 `plugin-world` 的 `.wasm` 组件放进目录即被
//! [`Host::discover`] 发现，`info()` 里的 name 就是注册名，宿主零配置文件。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::exports::spark::runtime::plugin::{PluginError, PluginInfo};
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

/// 长存宿主：共享 Engine + 组件编译缓存 + 一个 epoch bump 线程。
/// 每次调用新建独立 Store，沙箱隔离与资源上限不变；`Host` 方法取 `&self`，可多线程并发。
pub struct Host {
    engine: Engine,
    components: Mutex<HashMap<PathBuf, Arc<Component>>>,
    stop_bumper: Arc<AtomicBool>,
    bumper: Option<thread::JoinHandle<()>>,
}

impl Host {
    /// 新建宿主：编译引擎 + 启动 epoch bump 后台线程。
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config)?;

        let stop_bumper = Arc::new(AtomicBool::new(false));
        let bumper_engine = engine.clone();
        let bumper_stop = stop_bumper.clone();
        let bumper = thread::spawn(move || {
            while !bumper_stop.load(Ordering::Relaxed) {
                bumper_engine.increment_epoch();
                thread::sleep(Duration::from_millis(EPOCH_TICK_MS));
            }
        });

        Ok(Self {
            engine,
            components: Mutex::new(HashMap::new()),
            stop_bumper,
            bumper: Some(bumper),
        })
    }

    /// 组件编译缓存：同路径只编译一次（`Component::from_file` 每调用一次都是完整编译，贵）。
    fn component(&self, wasm_path: &str) -> Result<Arc<Component>> {
        let path = PathBuf::from(wasm_path);
        if let Some(c) = self.components.lock().unwrap().get(&path) {
            return Ok(c.clone());
        }
        let c = Arc::new(Component::from_file(&self.engine, wasm_path)?);
        self.components.lock().unwrap().insert(path, c.clone());
        Ok(c)
    }

    /// 调用一次 `info()` 与 `transform(input)`。
    /// 返回 `(插件信息, 变换结果)`；`transform` 的 `Err` 是插件声明式失败（结构化
    /// [`PluginError`]，值，非崩溃），外层 `Err` 才是宿主/沙箱错误（trap、加载失败、资源越限）。
    pub fn run(
        &self,
        wasm_path: &str,
        input: &str,
    ) -> Result<(PluginInfo, std::result::Result<String, PluginError>)> {
        let component = self.component(wasm_path)?;
        let linker = Linker::new(&self.engine);
        let mut store = new_store(&self.engine);
        let instance = PluginWorld::instantiate(&mut store, &component, &linker)?;
        let plugin = instance.spark_runtime_plugin();
        let info = plugin.call_info(&mut store)?;
        let out = plugin.call_transform(&mut store, input)?;
        Ok((info, out))
    }

    /// 仅沙箱内读取插件元数据（`info()`）：注册/发现用，不调用领域逻辑。
    pub fn info(&self, wasm_path: &str) -> Result<PluginInfo> {
        let component = self.component(wasm_path)?;
        let linker = Linker::new(&self.engine);
        let mut store = new_store(&self.engine);
        let instance = PluginWorld::instantiate(&mut store, &component, &linker)?;
        Ok(instance.spark_runtime_plugin().call_info(&mut store)?)
    }

    /// 发现 `dir` 下的插件：逐个沙箱读取 `info()`（组件编译进缓存），得到 `(文件名, 插件信息)`。
    /// 插件**自注册** —— name 就是注册名；读失败的组件跳过并在 stderr 提示，不中断整批。
    pub fn discover(&self, dir: &str) -> Vec<(String, PluginInfo)> {
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
            match self.info(&path.to_string_lossy()) {
                Ok(info) => found.push((name, info)),
                Err(e) => eprintln!("跳过 {name}：{e:#}"),
            }
        }
        found
    }

    /// 插件流水线：输入依次经过 `names` 各插件，前一步输出喂给下一步。
    /// 任一步声明式失败（`code` 可编程区分）或 trap 即 fail-fast。
    pub fn pipe(&self, dir: &str, input: &str, names: &[&str]) -> Result<String, PipeFailure> {
        let found = self.discover(dir);
        let mut current = input.to_string();
        for name in names {
            let Some((file, _)) = found.iter().find(|(_, info)| info.name == *name) else {
                return Err(PipeFailure::Declined {
                    step: (*name).to_string(),
                    error: PluginError {
                        code: "not-found".into(),
                        message: format!("未找到插件 `{name}`"),
                    },
                });
            };
            let path = format!("{dir}/{file}");
            match self.run(&path, &current) {
                Ok((_, Ok(out))) => current = out,
                Ok((_, Err(error))) => {
                    return Err(PipeFailure::Declined {
                        step: (*name).to_string(),
                        error,
                    });
                }
                Err(e) => {
                    return Err(PipeFailure::Trap {
                        step: (*name).to_string(),
                        detail: format!("{e:#}"),
                    });
                }
            }
        }
        Ok(current)
    }
}

/// 流水线失败：哪一步、怎么失败。
#[derive(Debug)]
pub enum PipeFailure {
    /// 插件声明式失败：`code` 供分支，`message` 供人读。
    Declined { step: String, error: PluginError },
    /// 沙箱/trap：插件崩溃被捕获，宿主存活。
    Trap { step: String, detail: String },
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stop_bumper.store(true, Ordering::Relaxed);
        if let Some(handle) = self.bumper.take() {
            handle.join().ok();
        }
    }
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
    store.set_epoch_deadline(2); // 当前 epoch +2：留一格裕量避开 bump 竞态，越界约 10–20ms 内切断
    store
}
