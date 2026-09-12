//! P6：Capability 后端落到**网络另一头**时，Component 的边界有没有变？
//!
//! 答案的骨架在这里：`counter-store.wasm` **一个字没改、一次没重编译**，
//! WIT 也没改 —— 只有 `Arc<dyn CapabilityBackend>` 背后站的是谁变了。
//!
//! 替身 HTTP 服务在测试进程内起（`TcpListener::bind("127.0.0.1:0")`），
//! 所以测试自包含、不依赖 node，也不依赖 `tools/remote-store-server.mjs`。
//!
//! 观测方式沿用 P2 的纪律：**新建实例读到什么**，而不是往契约里加探针。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use spark_host::domain_store::{click_times, CapabilityBackend, RemoteBackend, StoreError};
use spark_host::Host;

const NS: &str = "counter-store";
const SKIP: &str =
    "skip: 组件未构建，先 `cd components/counter-store && cargo component build --release`";

fn component_path() -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../components/counter-store/target/wasm32-unknown-unknown/release/counter_store.wasm",
    );
    p.exists().then(|| p.to_string_lossy().into_owned())
}

/// 进程内替身远端存储服务：`/storage/get`、`/storage/set`、`/stats`。
struct MockRemote {
    url: String,
    /// 服务端自己的状态。测试直接读它，是为了**独立于客户端**地看值落在哪。
    entries: Arc<Mutex<HashMap<String, String>>>,
}

fn read_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let head_end = loop {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();

    let content_len = head
        .lines()
        .find_map(|l| {
            let (name, value) = l.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    while buf.len() < head_end + content_len {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let body = String::from_utf8_lossy(&buf[head_end..]).into_owned();

    let mut parts = head.lines().next()?.split_whitespace();
    Some((parts.next()?.into(), parts.next()?.into(), body))
}

fn start_remote() -> MockRemote {
    start_remote_with(false)
}

/// `fail_set` = 替身版本的 `--fail-set`：每次 set 回 **500**。
/// 这是**服务实例的配置**，不是组件里的分支 —— 与 P5 的 `--deny` 同一条纪律。
fn start_remote_with(fail_set: bool) -> MockRemote {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let entries = Arc::new(Mutex::new(HashMap::<String, String>::new()));
    let shared = entries.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let Some((method, path, body)) = read_request(&mut stream) else {
                continue;
            };
            let (status, payload) = match (method.as_str(), path.as_str()) {
                ("GET", "/stats") => (
                    200,
                    json!({ "entries": shared.lock().unwrap().len() }).to_string(),
                ),
                ("POST", "/storage/get") => {
                    let body: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    let value = body["key"]
                        .as_str()
                        .and_then(|k| shared.lock().unwrap().get(k).cloned());
                    // 缺 key → null（不是失败）；契约要求「缺失」与「失败」可区分。
                    (200, json!({ "value": value }).to_string())
                }
                ("POST", "/storage/set") if fail_set => {
                    (500, r#"{"error":"set disabled by --fail-set"}"#.to_string())
                }
                ("POST", "/storage/set") => {
                    let body: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    if let (Some(k), Some(v)) = (body["key"].as_str(), body["value"].as_str()) {
                        shared.lock().unwrap().insert(k.into(), v.into());
                    }
                    (200, "{}".to_string())
                }
                _ => (404, "{}".to_string()),
            };
            let resp = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    MockRemote { url, entries }
}

/// 状态真的落在**网络另一头**：客户端侧换一个新的 `RemoteBackend` 对象（没有任何本地状态），
/// 仍然读得到上一次写进去的值。
#[test]
fn state_lives_in_the_remote_not_in_the_client() {
    let Some(wasm) = component_path() else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();
    let remote = start_remote();

    // 第一次：写入 3
    let backend = RemoteBackend::new(&remote.url);
    assert_eq!(click_times(&host, &wasm, 3, NS, backend).unwrap(), 3);

    // 独立于组件的观测：值确实到了服务端，key 是**加了 Host 前缀**的裸 key。
    assert_eq!(
        remote
            .entries
            .lock()
            .unwrap()
            .get("counter-store:count")
            .map(String::as_str),
        Some("3")
    );

    // 一个全新的客户端对象（同一个远端）—— 它自己没有任何状态，却读到了 3。
    let fresh_client = RemoteBackend::new(&remote.url);
    assert_eq!(click_times(&host, &wasm, 0, NS, fresh_client).unwrap(), 3);

    // 再点 3 次：起点是远端的 3，不是本地新开的 0。
    let again = RemoteBackend::new(&remote.url);
    assert_eq!(click_times(&host, &wasm, 3, NS, again).unwrap(), 6);
}

/// 网络失败是 `unavailable`，**不是** `denied` —— `denied` 在 P2 已被冻结为
/// 「宿主侧只读模式」，拿它接网络失败等于静默重定义那个 variant。
///
/// 两条失败臂都断言：① 连不上 ② 远端回 5xx。
/// **本轮不主张这两者应当被区分** —— 只主张两者都**不得**是 `denied`。
///
/// 断言直接打在 `RemoteBackend` 上，**不经组件**：组件吞掉 `Err`
/// （`components/counter-store/src/lib.rs` 的 `let _ = storage::set(...)`），
/// 所以错误值从组件那侧根本不可观测 —— P2 的说法是
/// *set failure is observable through a subsequent fresh instance*。
#[test]
fn remote_failure_is_unavailable_not_denied() {
    let unreachable = RemoteBackend::new("http://127.0.0.1:1");
    match unreachable.set("k", "v") {
        Err(StoreError::Unavailable(msg)) => assert!(!msg.is_empty()),
        other => panic!("连不上应当是 Unavailable，实际是 {other:?}"),
    }
    match unreachable.get("k") {
        Err(StoreError::Unavailable(_)) => {}
        other => panic!("连不上应当是 Unavailable，实际是 {other:?}"),
    }

    let failing = start_remote_with(true);
    let backend = RemoteBackend::new(&failing.url);
    match backend.set("k", "v") {
        Err(StoreError::Unavailable(msg)) => assert!(msg.contains("500"), "应指明远端状态: {msg}"),
        other => panic!("远端 5xx 应当是 Unavailable，实际是 {other:?}"),
    }
    // 只禁止 `set` 的替身：`get` 仍应正常，证明这条臂不是「整个后端坏了」。
    match backend.get("k") {
        Ok(None) => {}
        other => panic!("--fail-set 只该禁 write，get 应当正常回 None，实际是 {other:?}"),
    }
}

/// `base_url` 真的在选择状态 —— 不是被忽略的装饰。
#[test]
fn remote_base_url_selects_the_state() {
    let Some(wasm) = component_path() else {
        eprintln!("{SKIP}");
        return;
    };
    let host = Host::new().unwrap();
    let a = start_remote();
    let b = start_remote();

    assert_eq!(
        click_times(&host, &wasm, 3, NS, RemoteBackend::new(&a.url)).unwrap(),
        3
    );
    // 换一个**空**的远端：同一个 artifact、同一个 Host、同一个 ns、同一个 click protocol。
    assert_eq!(
        click_times(&host, &wasm, 0, NS, RemoteBackend::new(&b.url)).unwrap(),
        0
    );
    // 回到 a：3 还在。
    assert_eq!(
        click_times(&host, &wasm, 0, NS, RemoteBackend::new(&a.url)).unwrap(),
        3
    );
}
