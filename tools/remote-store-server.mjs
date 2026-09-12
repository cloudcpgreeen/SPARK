#!/usr/bin/env node
// P6 的 stand-in 远端服务：把 `spark:capability/storage@0.1.0` 的实现放到**另一个操作系统进程**里。
//
// 它是替身，不是产品：单进程内存态、无鉴权、无并发控制、没有磁盘。
// ponytail: 内存 Map + node:http 标准库就够撞「remote 到底是不是 Host 的事」；
//           要真做产品就换真存储 + 鉴权，别往这个文件上加。
//
// 用另一种语言/runtime 写是刻意的 —— 对面是什么，组件根本不知道。
//
//   node tools/remote-store-server.mjs [--port N] [--fail-set]
//
// `--fail-set` 让每次 set 都回 500。这是**远端服务实例的配置**，不是组件里的分支
// —— 与 P5 的 `--deny` 同一条纪律：失败配置在宿主/服务这一侧，不在 Domain 里。

import { createServer } from 'node:http';

const argv = process.argv.slice(2);
const failSet = argv.includes('--fail-set');
const portArg = argv.indexOf('--port');
const port = portArg === -1 ? 4318 : Number(argv[portArg + 1]);

const state = new Map();

function json(res, code, body) {
  const payload = JSON.stringify(body);
  res.writeHead(code, {
    'content-type': 'application/json',
    'content-length': Buffer.byteLength(payload),
  });
  res.end(payload);
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let raw = '';
    req.setEncoding('utf8');
    req.on('data', (c) => {
      raw += c;
    });
    req.on('end', () => {
      try {
        resolve(raw ? JSON.parse(raw) : {});
      } catch (e) {
        reject(e);
      }
    });
    req.on('error', reject);
  });
}

const server = createServer(async (req, res) => {
  const { pathname } = new URL(req.url, 'http://localhost');
  try {
    if (req.method === 'GET' && pathname === '/stats') {
      return json(res, 200, { entries: state.size });
    }
    if (req.method !== 'POST') {
      return json(res, 405, { error: 'POST only' });
    }

    const body = await readBody(req);

    if (pathname === '/storage/get') {
      const v = state.get(body.key);
      // 缺少这个 key 不是失败 —— 契约要求「缺失」与「失败」可区分。
      return json(res, 200, { value: v === undefined ? null : v });
    }

    if (pathname === '/storage/set') {
      if (failSet) {
        return json(res, 500, { error: 'set disabled by --fail-set' });
      }
      state.set(body.key, body.value);
      return json(res, 200, {});
    }

    return json(res, 404, { error: `no such endpoint: ${pathname}` });
  } catch (e) {
    return json(res, 400, { error: String(e) });
  }
});

server.listen(port, '127.0.0.1', () => {
  const { port: actual } = server.address();
  console.log(
    `remote-store listening on http://127.0.0.1:${actual}${failSet ? ' (fail-set)' : ''}`,
  );
});
