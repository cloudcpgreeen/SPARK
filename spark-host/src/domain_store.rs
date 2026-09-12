//! P2 Capability Contract Spike：为 `spark:capability/storage@0.1.0` 提供**后端实现**。
//!
//! 契约见 `wit/store.wit`（`spark:store@0.1.0`）。组件只声明 import，实现完全在这里——
//! 换一个介质（进程内 map → 真实 DB）只需要改这一个文件，**组件不重新编译**。
//!
//! **key 命名空间是 Host 的实例策略，不是契约的一部分。** 组件发的是裸 key（`"count"`），
//! 前缀由本模块在实例化时按 `ns` 加上。契约只说「按 key 读写」；谁和谁不共享数据是宿主的事。
//! 前缀跟着 **Host** 走，不跟着介质走 —— 远端后端发到线上的也是加好前缀的 key。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use serde_json::{json, Value};
use wasmtime::component::{HasSelf, Linker};
use wasmtime::{Store, StoreLimits};

use crate::{sandbox_limits, Host};

wasmtime::component::bindgen!({
    path: "wit-store",
    world: "store-world",
});

/// 契约的错误值：`unavailable(string)` / `denied(string)`。
/// `denied` 在 P2 已被冻结为「宿主侧只读模式」，**不能**拿来接网络失败。
pub use spark::capability::storage::StoreError;

/// Capability 后端：契约的三个动作。实现可以落在**进程内**，也可以落在**网络另一头** ——
/// 组件对这两者一无所知。
pub trait CapabilityBackend {
    fn get(&self, key: &str) -> std::result::Result<Option<String>, StoreError>;
    fn set(&self, key: &str, value: &str) -> std::result::Result<(), StoreError>;
    /// 后端自己持有的条目数。本地就是 `len()`；远端得问对面 —— 这是**独立于组件**的观测。
    fn stored(&self) -> std::result::Result<usize, String>;
}

/// 后端侧的 capability 实现：进程内 map。换介质就改这个文件。
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

impl CapabilityBackend for Backend {
    fn get(&self, key: &str) -> std::result::Result<Option<String>, StoreError> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> std::result::Result<(), StoreError> {
        if self.deny_writes {
            return Err(StoreError::Denied("read-only backend".into()));
        }
        self.data
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn stored(&self) -> std::result::Result<usize, String> {
        Ok(self.len())
    }
}

/// Capability 后端：**网络另一头**的存储服务。
///
/// 同步 WIT 之所以能走网络，靠的是**阻塞** —— `ureq` 在 wasmtime 的调用线程上等，
/// 返回后照常 lowering。这里没有任何 async，也不需要。代价是：只有能为同步调用
/// 提供阻塞式 IO 的 Host 才装得上这个后端（Web/jco 侧就不是）。
pub struct RemoteBackend {
    base_url: String,
    timeout: Duration,
}

impl RemoteBackend {
    pub fn new(base_url: &str) -> Arc<Self> {
        Arc::new(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            // 不设超时的话，指向黑洞地址会一直挂着。
            timeout: Duration::from_secs(5),
        })
    }

    fn post(&self, path: &str, body: Value) -> std::result::Result<Value, StoreError> {
        let url = format!("{}{path}", self.base_url);
        match ureq::post(&url).timeout(self.timeout).send_json(body) {
            Ok(resp) => resp
                .into_json::<Value>()
                .map_err(|e| StoreError::Unavailable(format!("{url}: 响应不是 JSON: {e}"))),
            // 连不上、DNS 失败、超时 → 传输层
            Err(ureq::Error::Transport(t)) => {
                Err(StoreError::Unavailable(format!("{url}: 不可达: {t}")))
            }
            // 远端回了个非 2xx
            Err(ureq::Error::Status(code, _)) => {
                Err(StoreError::Unavailable(format!("{url}: 远端状态 {code}")))
            }
        }
    }
}

impl CapabilityBackend for RemoteBackend {
    fn get(&self, key: &str) -> std::result::Result<Option<String>, StoreError> {
        let resp = self.post("/storage/get", json!({ "key": key }))?;
        Ok(match resp.get("value") {
            Some(Value::String(s)) => Some(s.clone()),
            // `null` 与「没有这个字段」都是「没有这个 key」——缺失不是失败。
            _ => None,
        })
    }

    fn set(&self, key: &str, value: &str) -> std::result::Result<(), StoreError> {
        self.post("/storage/set", json!({ "key": key, "value": value }))?;
        Ok(())
    }

    fn stored(&self) -> std::result::Result<usize, String> {
        let url = format!("{}/stats", self.base_url);
        let resp = ureq::get(&url)
            .timeout(self.timeout)
            .call()
            .map_err(|e| format!("不可达: {e}"))?;
        let body = resp
            .into_json::<Value>()
            .map_err(|e| format!("响应不是 JSON: {e}"))?;
        body.get("entries")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .ok_or_else(|| "响应里没有 entries".to_string())
    }
}

struct StoreData {
    limits: StoreLimits,
    /// 本实例的命名空间（Host 策略，非契约语义）。
    ns: String,
    backend: Arc<dyn CapabilityBackend>,
}

impl spark::capability::storage::Host for StoreData {
    fn get(&mut self, key: String) -> Result<Option<String>, StoreError> {
        self.backend.get(&scoped(&self.ns, &key))
    }

    fn set(&mut self, key: String, value: String) -> Result<(), StoreError> {
        self.backend.set(&scoped(&self.ns, &key), &value)
    }
}

/// Host 侧命名空间策略：同一后端被多个组件复用时靠它隔开。
fn scoped(ns: &str, key: &str) -> String {
    format!("{ns}:{key}")
}

fn new_store_data(
    engine: &wasmtime::Engine,
    ns: &str,
    backend: Arc<dyn CapabilityBackend>,
) -> Store<StoreData> {
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
///
/// **epoch 预算是 per-call 的**：`new_store_data` 只设一次起点，纯本地调用微秒级所以从未暴露
/// 问题，但一次阻塞的远端 `set` 就可能吃掉整个预算 —— 而超时表现为 `store trap:`，
/// 退出码仍是 SUCCESS，证据会被静默污染。所以每次 `call_*` 前重设起点。
/// 这对既有断言只放宽不放严（死循环仍在 ~10–20ms 内被切断）。
///
/// `domain.rs::click_times` 与 `compose.rs::click_times_selfcontained` 形状相同但纯本地、
/// 从未触发，**未同步修改** —— 那是独立债务，见本轮记录。
pub fn click_times(
    host: &Host,
    wasm_path: &str,
    clicks: u32,
    ns: &str,
    backend: Arc<dyn CapabilityBackend>,
) -> Result<u32> {
    let component = host.component(wasm_path)?;
    let mut linker = Linker::new(&host.engine);
    StoreWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;

    let mut store = new_store_data(&host.engine, ns, backend);
    let instance = StoreWorld::instantiate(&mut store, &component, &linker)?;

    let counter = instance.spark_store_counter_store().counter();
    store.set_epoch_deadline(2);
    let handle = counter.call_constructor(&mut store)?;
    for _ in 0..clicks {
        store.set_epoch_deadline(2);
        counter.call_click(&mut store, handle)?;
    }
    store.set_epoch_deadline(2);
    Ok(counter.call_count(&mut store, handle)?)
}
