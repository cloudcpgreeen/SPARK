//! P3 的 Provider Component：**实现** `spark:capability/storage@0.1.0`，零 import。
//!
//! 它把键值放在自己的实例内存里。这既是它「零 import」的原因，也是它的作用域的来源：
//! 状态住在 Provider 实例内，所以组合后能力状态是 instance-scoped。
//! 这是**本次实现的后果**，不是 `spark:capability/storage` 契约的规定。

mod bindings;

use std::cell::RefCell;
use std::collections::HashMap;

use bindings::exports::spark::capability::storage::{Guest, StoreError};

// storage 是**按值导出**的接口（不是 resource）：Guest 方法是静态的，拿不到 &self。
// 状态因此挂在这个实例的模块静态上 —— 作用域仍然是「一个 Provider 实例」。
thread_local! {
    static MAP: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

struct MemStore;

impl Guest for MemStore {
    fn get(key: String) -> Result<Option<String>, StoreError> {
        MAP.with(|m| Ok(m.borrow().get(&key).cloned()))
    }

    fn set(key: String, value: String) -> Result<(), StoreError> {
        MAP.with(|m| m.borrow_mut().insert(key, value));
        Ok(())
    }
}

bindings::export!(MemStore with_types_in bindings);
