// Web Host 对 `spark:capability/storage@0.1.0` 的实现：浏览器 localStorage。
//
// jco 的约定：**成功就正常返回，失败要 throw 出 WIT 的 error 值**
// （`throw { tag: 'denied', val: ... }`）。返回 `{ tag: 'err', ... }` 会被当成成功——踩过。
//
// 无痕模式 / 存储被禁时 localStorage 本身就会抛异常，所以这里不需要假的开关：
// 真实的浏览器环境天然就是这个能力的错误路径。

// 命名空间是 **Host 的实例策略**，不是契约语义：组件发的是裸 key。
const NS = 'counter-store';

const k = (key) => `${NS}:${key}`;

function guard(fn) {
  try {
    return fn();
  } catch {
    throw { tag: 'denied', val: 'storage unavailable (private mode?)' };
  }
}

export function get(key) {
  return guard(() => {
    const v = localStorage.getItem(k(key));
    // undefined = 没有这个 key（Ok(None)）；抛错才是能力失败（Err）。
    return v === null ? undefined : v;
  });
}

export function set(key, value) {
  guard(() => localStorage.setItem(k(key), value));
}
