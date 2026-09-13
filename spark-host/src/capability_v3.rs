//! G7：`spark:capability/storage@0.3.0` 的 host 侧 binding —— G6 选中的 `future<T>` 形状。
//!
//! 这里**只有 binding 生成，没有实现**。G7 的交付面停在「新契约能被 wasmtime 的 bindgen
//! 展开、且生成物在本仓库的工具链里编译得过」；`impl …::HostWithStore` 属于下一阶段。
//! 之所以要有这个模块而不是只留一个 `.wit`：契约是否能被 host 侧消费，
//! 只有让 bindgen 真的跑一遍才算证据。
//!
//! `imports: { …: store }` 是 **Host 侧 binding 选项**（WIT 一字未动）：选了它，
//! `get` 拿的是 `Access<T, Self>`，从而能 `FutureReader::new(store, producer)`；
//! 不选它，生成的方法签名里没有 store，那个 future 结构上就构造不出来。
//! 这正是 G6-B 记的「约束搬家」—— 从 `async func` 的全局 `Send` 换成一处显式 binding。

wasmtime::component::bindgen!({
    path: "wit-store-v3",
    world: "store-v3-world",
    imports: { "spark:capability/storage@0.3.0": store },
});
