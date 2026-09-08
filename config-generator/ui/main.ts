import { mount, unmount } from 'svelte';
import init from '../pkg/engine';
import wasmUrl from '../pkg/engine_bg.wasm?url';
import App from './App.svelte';
import { Controller } from './controller.svelte';
import '../styles.css';

async function start() {
  const loading = document.getElementById('cg-loading');
  try {
    await init({ module_or_path: wasmUrl });
    const target = document.getElementById('app');
    if (!target) throw new Error('找不到应用容器。');
    const controller = new Controller();
    const app = mount(App, { target, props: { controller } });
    // App component hot replacement reuses its controller; only the owner frees WASM.
    import.meta.hot?.dispose(() => {
      void unmount(app);
      controller.dispose();
    });
    loading?.remove();
  } catch {
    if (loading) {
      loading.setAttribute('role', 'alert');
      loading.textContent = '配置生成器加载失败，请刷新页面并确认浏览器支持 WebAssembly。';
    }
  }
}

void start();
