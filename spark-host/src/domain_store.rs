//! P2 Capability Contract Spike：为 `spark:capability/storage@0.1.0` 提供**后端实现**。
//!
//! 契约见 `wit/store.wit`（`spark:store@0.1.0`）。组件只声明 import，实现完全在这里——
//! 换一个介质（进程内 map → 真实 DB）只需要改这一个文件，**组件不重新编译**。
//!
//! **key 命名空间是 Host 的实例策略，不是契约的一部分。** 组件发的是裸 key（`"count"`），
//! 前缀由本模块在实例化时按 `ns` 加上。契约只说「按 key 读写」；谁和谁不共享数据是宿主的事。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use wasmtime::component::{HasSelf, Linker};
use wasmtime::{Store, StoreLimits};

use crate::{sandbox_limits, Host};

wasmtime::component::bindgen!({
    path: "wit-store",
    world: "store-world",
});

/// 后端侧的 capability 实现：进程内 map。换 DB 就改这里。
pub struct Backend {
    data: Mutex<HashMap<String, String>>,
    deny_writes: bool,
}

impl Backend {
    /// 健康存储。
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            data: Mutex::new(HashMap::new()),
            deny_writes: false,
        })
    }

    /// 拒绝写入的存储：`set` 返回 `denied`，`get` 正常。用来观察错误路径。
    pub fn read_only() -> Arc<Self> {
        Arc::new(Self {
            data: Mutex::new(HashMap::new()),
            deny_writes: true,
        })
    }

    /// 实际落盘的条目数。
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

struct StoreData {
    limits: StoreLimits,
    /// 本实例的命名空间（Host 策略，非契约语义）。
    ns: String,
    backend: Arc<Backend>,
}

impl spark::capability::storage::Host for StoreData {
    fn get(
        &mut self,
        key: String,
    ) -> Result<Option<String>, spark::capability::storage::StoreError> {
        Ok(self
            .backend
            .data
            .lock()
            .unwrap()
            .get(&scoped(&self.ns, &key))
            .cloned())
    }

    fn set(
        &mut self,
        key: String,
        value: String,
    ) -> Result<(), spark::capability::storage::StoreError> {
        if self.backend.deny_writes {
            return Err(spark::capability::storage::StoreError::Denied(
                "read-only backend".into(),
            ));
        }
        self.backend
            .data
            .lock()
            .unwrap()
            .insert(scoped(&self.ns, &key), value);
        Ok(())
    }
}

/// Host 侧命名空间策略：同一后端被多个组件复用时靠它隔开。
fn scoped(ns: &str, key: &str) -> String {
    format!("{ns}:{key}")
}

fn new_store_data(engine: &wasmtime::Engine, ns: &str, backend: Arc<Backend>) -> Store<StoreData> {
    let mut store = Store::new(
        engine,
        StoreData {
            limits: sandbox_limits(),
            ns: ns.to_string(),
            backend,
        },
    );
    store.limiter(|data| &mut data.limits);
    store.set_epoch_deadline(2); // 与 new_store 同一套安全模式，见 lib.rs::sandbox_limits
    store
}

/// 新建一个 counter 实例，点 `clicks` 次，返回它自己看到的 count。
///
/// `clicks = 0` 就是「一个全新实例读到了什么」——P2 观察 capability 是否生效的方式。
pub fn click_times(
    host: &Host,
    wasm_path: &str,
    clicks: u32,
    ns: &str,
    backend: Arc<Backend>,
) -> Result<u32> {
    let component = host.component(wasm_path)?;
    let mut linker = Linker::new(&host.engine);
    StoreWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;

    let mut store = new_store_data(&host.engine, ns, backend);
    let instance = StoreWorld::instantiate(&mut store, &component, &linker)?;

    let counter = instance.spark_store_counter_store().counter();
    let handle = counter.call_constructor(&mut store)?;
    for _ in 0..clicks {
        counter.call_click(&mut store, handle)?;
    }
    Ok(counter.call_count(&mut store, handle)?)
}
