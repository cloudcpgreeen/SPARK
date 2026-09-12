//! P4-0 preflight 的实验对象：一个**同时 import 与 export** `spark:capability/storage@0.1.0`
//! 的 Provider 组件。
//!
//! 它自己**不做存储** —— 收到 `get`/`set` 就原样转发给上一层（它 import 的那一个）。
//! 所以它没什么「状态」可言，作用域问题在这个组件里被彻底交出去了：
//! 这正是 P4 要问的「Provider 能否通过另一个 Capability 拿到跨 instance 的状态」。
//!
//! P4-0 阶段只关心**它能不能被构建出来**。转发是不是真的接得上（委派给谁），是 P4-1 的事。

mod bindings;

use bindings::exports::spark::capability::storage::{Guest, StoreError};
use bindings::spark::capability::storage as upstream;

struct DelegatingStore;

/// **同一个 interface 的 import 侧与 export 侧是两个不同的 Rust 类型。**
/// `bindings::spark::capability::storage::StoreError` 与
/// `bindings::exports::spark::capability::storage::StoreError` 结构相同、名字相同，
/// 但类型不同 —— 所以委派必然要在值这一层做一次转换。
/// 这不是工具链的缺陷，是「导入的接口」与「导出的接口」本就不该被当成同一个东西。
fn relay(e: upstream::StoreError) -> StoreError {
    match e {
        upstream::StoreError::Unavailable(m) => StoreError::Unavailable(m),
        upstream::StoreError::Denied(m) => StoreError::Denied(m),
    }
}

impl Guest for DelegatingStore {
    fn get(key: String) -> Result<Option<String>, StoreError> {
        upstream::get(&key).map_err(relay)
    }

    fn set(key: String, value: String) -> Result<(), StoreError> {
        upstream::set(&key, &value).map_err(relay)
    }
}

bindings::export!(DelegatingStore with_types_in bindings);
