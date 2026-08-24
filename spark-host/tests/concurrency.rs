//! 并发：共享宿主（Host）上多线程并行调用，编译缓存复用、每调用独立 Store，trap 不串。

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use spark_host::Host;

/// 解开 Ok；失败则 panic（PluginError 未实现 PartialEq，不能直接 assert_eq 整个 Result）。
fn expect_ok<T, E>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(_) => panic!("期望成功，实际失败"),
    }
}

fn built_wasm(plugin_dir: &str) -> Option<PathBuf> {
    let wasm_name = format!("{}.wasm", plugin_dir.replace('-', "_"));
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../{plugin_dir}/target/wasm32-unknown-unknown/release/{wasm_name}"));
    p.exists().then_some(p)
}

#[test]
fn concurrent_calls_share_host_and_stay_isolated() {
    let Some(upper) = built_wasm("spark-plugin") else {
        eprintln!("skip: 组件未构建");
        return;
    };
    let Some(reverse) = built_wasm("spark-plugin-reverse") else {
        eprintln!("skip: reverse 未构建");
        return;
    };
    let host = Arc::new(Host::new().unwrap());
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let (host, upper, reverse) = (host.clone(), upper.clone(), reverse.clone());
            thread::spawn(move || {
                if i == 0 {
                    // 一个线程触发 trap：共享宿主上隔离不破，其余线程不受影响。
                    assert!(host.run(&upper.to_string_lossy(), "trap-x").is_err());
                }
                let (_, out) = host.run(&upper.to_string_lossy(), "hello").unwrap();
                assert_eq!(expect_ok(out), "HELLO");
                let (_, out2) = host.run(&reverse.to_string_lossy(), "hi").unwrap();
                assert_eq!(expect_ok(out2), "ih");
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}
