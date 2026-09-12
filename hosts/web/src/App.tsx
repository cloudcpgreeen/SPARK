import { useReducer, useState } from 'react';

import { button } from './generated/button/button.js';
import { counterStore } from './generated/composed/composed.js';
import { counterStore as hostedCounterStore } from './generated/store/counter-store.js';

export default function App() {
  // 唯一的 React 状态：组件实例的引用。count 不在 React 里。
  const [counter] = useState(() => new button.Counter());
  // 点击后只做一件事：让 React 重渲染。它不持有任何业务状态。
  const [, rerender] = useReducer((n: number) => n + 1, 0);

  // 每次渲染都向组件要值 —— 这个数字的唯一来源是 wasm 组件。
  const count = counter.count();

  // P2：同一份 counter-store.wasm，capability 由本 Host 用 localStorage 实现。
  // 状态住在 capability 里，所以刷新后还在。
  const [stored] = useState(() => new hostedCounterStore.Counter());
  const [, rerenderStored] = useReducer((n: number) => n + 1, 0);
  const storedCount = stored.count();

  // P3：capability 的实现也是一个 Component（mem-store），由 wasm-tools compose 拼进来。
  // import 已被组合消掉，所以这一份**没有 --map**、不依赖任何 Host 注入。
  // Provider 把状态放在自己的实例里 → 作用域就是实例，刷新归零。
  const [composedCounter] = useState(() => new counterStore.Counter());
  const [, rerenderComposed] = useReducer((n: number) => n + 1, 0);
  const composedCount = composedCounter.count();

  return (
    <main>
      <h1>SPARK</h1>

      <section>
        <h2>P1 · domain-world</h2>
        <p>
          count: <strong>{count}</strong>
        </p>
        <button
          onClick={() => {
            counter.click();
            rerender();
          }}
        >
          click
        </button>
        <p>
          <small>
            数字住在 <code>button.wasm</code> 的实例里。刷新 → 归零。
          </small>
        </p>
      </section>

      <section>
        <h2>P2 · store-world · capability</h2>
        <p>
          count: <strong>{storedCount}</strong>
        </p>
        <button
          onClick={() => {
            stored.click();
            rerenderStored();
          }}
        >
          click
        </button>
        <p>
          <small>
            数字经 <code>spark:capability/storage</code> 落在 localStorage 里。刷新 → 还在。
            <br />
            同一份 <code>counter-store.wasm</code>；换实现只改 Host，组件不重新编译。
          </small>
        </p>
      </section>

      <section>
        <h2>P3 · composed · capability 由组件实现</h2>
        <p>
          count: <strong>{composedCount}</strong>
        </p>
        <button
          onClick={() => {
            composedCounter.click();
            rerenderComposed();
          }}
        >
          click
        </button>
        <p>
          <small>
            同一份 <code>counter-store.wasm</code>（未重新编译），由 <code>wasm-tools compose</code>{' '}
            把实现 capability 的 <code>mem-store.wasm</code> 组合进来。
            <br />
            import 已被组合消掉 —— 这一份**没有 Host 注入**。Provider 的状态住在自己的实例里，
            所以刷新 → 归零。
          </small>
        </p>
      </section>
    </main>
  );
}
