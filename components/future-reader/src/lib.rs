// P7-P4.1：B 路线（wit-bindgen + wasm-tools）在仓库内产出的第一个 async Component。
//
// 世界形状与 G6-A 的探针组件相同（import `0.3.0`、export `run: async func`），
// 这样它可以直接喂给 G8 那条已证实的 Wasmtime Host 链路做验收 ——
// 首轮要验证的是**构建路径**，不是 runtime。
//
// 与 G6-A 的差别只有一处：这里的一切都在仓库内、由 build-async.sh 产出。
wit_bindgen::generate!({ world: "future-reader-world", path: "wit", generate_all, features: ["async"] });

struct C;

impl Guest for C {
    async fn run(key: String) -> String {
        // 契约是 `func -> future<T>`：调用同步拿到句柄，挂起发生在随后的 await。
        let handle = spark::capability::storage::get(&key);
        match handle.await {
            Ok(Some(v)) => v,
            Ok(None) => String::from("<absent>"),
            Err(e) => format!("<err:{e:?}>"),
        }
    }
}

export!(C);
