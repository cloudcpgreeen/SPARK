// P7-P4.2 · ①b：仓库内、B 专用的 Component Verification Entry。
//
// 它消费 build-async.sh 刚从 fresh clone 产出的 future_reader.component.wasm，
// 走一遍 0.3.0 future<T> 的真实 Wasmtime 链路（注册 → 调用 → 宿主 store 的值返回）。
//
// 与 G8 探针 host 的差别只有两处，都是刻意的：
//   ① 删掉 Marker / AccessorTask / host.spawn —— 那是 G8 用来演示「宿主能 spawn」的，
//      不属于本 entry 的验收内容；
//   ② 组件路径没有默认值 —— 必须由调用方把刚产出的路径交进来，
//      这样「引用预生成的 .wasm / 复制进仓库的 golden artifact」在结构上就不可能。
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use wasmtime::component::{Access, Component, FutureReader, HasSelf, Linker, Val};
use wasmtime::{Config, Engine, Result, Store};

wasmtime::component::bindgen!({
    path: "wit",
    world: "w",
    imports: { "spark:capability/storage@0.3.0": store },
});

use spark::capability::storage::{HostWithStore, StoreError};

struct StoreData {
    map: Arc<Mutex<HashMap<String, String>>>,
}

impl spark::capability::storage::Host for StoreData {}

impl HostWithStore<StoreData> for HasSelf<StoreData> {
    fn get(
        mut host: Access<StoreData, Self>,
        key: String,
    ) -> FutureReader<std::result::Result<Option<String>, StoreError>> {
        let map = host.get().map.clone();
        let producer = async move {
            let v = map.lock().unwrap().get(&key).cloned();
            Ok::<std::result::Result<Option<String>, StoreError>, wasmtime::Error>(Ok(v))
        };
        FutureReader::new(host, producer).expect("FutureReader::new")
    }

    fn set(
        mut host: Access<StoreData, Self>,
        key: String,
        value: String,
    ) -> FutureReader<std::result::Result<(), StoreError>> {
        let map = host.get().map.clone();
        let producer = async move {
            map.lock().unwrap().insert(key, value);
            Ok::<std::result::Result<(), StoreError>, wasmtime::Error>(Ok(()))
        };
        FutureReader::new(host, producer).expect("FutureReader::new")
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("VERIFY_COMPONENT").ok())
        .unwrap_or_else(|| {
            eprintln!("usage: verify-async-component <path-to-future_reader.component.wasm>");
            std::process::exit(2)
        });

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;

    let component = Component::from_file(&engine, &path)?;
    println!("verify: component-loaded = {path}");

    let mut linker = Linker::<StoreData>::new(&engine);
    W::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    println!("verify: add_to_linker OK  <- 0.3.0 已注册");

    let map = Arc::new(Mutex::new(HashMap::new()));
    map.lock()
        .unwrap()
        .insert("count".to_string(), "42".to_string());
    let mut store = Store::new(&engine, StoreData { map });

    let mut results = [Val::String(String::new())];
    futures::executor::block_on(async {
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let run = instance.get_func(&mut store, "run").expect("export run");
        let args = [Val::String("count".to_string())];
        store
            .run_concurrent(async |accessor| -> Result<()> {
                run.call_concurrent(accessor, &args, &mut results).await?;
                Ok(())
            })
            .await??;
        Ok::<(), wasmtime::Error>(())
    })?;

    println!("verify: component-returned = {:?}", results[0]);
    assert!(
        matches!(results[0], Val::String(ref s) if s == "42"),
        "期望 42"
    );
    println!("VERIFY PASS: 0.3.0 future<T> 组件从仓库 verification host 的 store 读回了值");
    Ok(())
}
