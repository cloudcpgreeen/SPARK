//! P4-2 的 Provider B：与 `delegating-store` **语义逐字相同**的纯委派 Provider。
//!
//! 它自己**不做存储** —— 收到 `get`/`set` 就原样转发给上一层（它 import 的那一个）。
//! 存在的唯一理由是**在委派链上再叠一层**：P4-1 的 A 在内、B 在外。
//!
//! 刻意与 `delegating-store` 保持复制关系而不是抽公共 crate：只有两个实例，
//! 抽象的成本（多一个 crate、多一层依赖）比复制的成本高。二者一旦分化，说明 P4-2
//! 的实验控制被破坏了 —— 这正是它们必须**逐字相同**的原因。

mod bindings;

use bindings::exports::spark::capability::storage::{Guest, StoreError};
use bindings::spark::capability::storage as upstream;

struct ForwardingStore;

/// **同一个 interface 的 import 侧与 export 侧是两个不同的 Rust 类型**（P4-0 的发现）。
/// 两侧名字相同、结构相同，但类型不同，所以委派必然要在值这一层转换一次。
fn relay(e: upstream::StoreError) -> StoreError {
    match e {
        upstream::StoreError::Unavailable(m) => StoreError::Unavailable(m),
        upstream::StoreError::Denied(m) => StoreError::Denied(m),
    }
}

impl Guest for ForwardingStore {
    fn get(key: String) -> Result<Option<String>, StoreError> {
        upstream::get(&key).map_err(relay)
    }

    fn set(key: String, value: String) -> Result<(), StoreError> {
        upstream::set(&key, &value).map_err(relay)
    }
}

bindings::export!(ForwardingStore with_types_in bindings);
