import { useReducer, useState } from 'react';

import { button } from './generated/button/button.js';
import { counterStore } from './generated/store/counter-store.js';

export default function App() {
  // 唯一的 React 状态：组件实例的引用。count 不在 React 里。
  const [counter] = useState(() => new button.Counter());
  // 点击后只做一件事：让 React 重渲染。它不持有任何业务状态。
  const [, rerender] = useReducer((n: number) => n + 1, 0);

  // 每次渲染都向组件要值 —— 这个数字的唯一来源是 wasm 组件。
  const count = counter.count();

  // P2：同一份 counter-store.wasm，capability 由本 Host 用 localStorage 实现。
  // 状态住在 capability 里，所以刷新后还在。
  const [stored] = useState(() => new counterStore.Counter());
  const [, rerenderStored] = useReducer((n: number) => n + 1, 0);
  const storedCount = stored.count();

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
    </main>
  );
}
