import { initialState, initialForward, credentials, VERSION, MODES, FORMATS, TLS_MODES, CONTROLLERS, LOG_LEVELS, INPUT, USER, FORWARD, hasClient, hasServer } from './schema.mjs';
import { active } from './dsl.mjs';
import { validate, endpoint } from './validation.mjs';
import { buildConfigs, redactConfig } from './model.mjs';
import { serialize } from './serializers.mjs';

function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key.startsWith('on')) node.addEventListener(key.slice(2), value);
    else if (key === 'className') node.className = value;
    else node.setAttribute(key, String(value));
  }
  for (const child of children.flat()) if (child != null) node.append(child);
  return node;
}

export function initialize(root) {
  if (!root || root.dataset.ready) return;
  document.body.classList.add('cg-page');
  root.dataset.ready = 'true';
  let state = initialState();
  let outputSide = 'server';
  let showSecrets = false;
  let currentText = '';
  const inputs = new Map();
  const errorNodes = new Map();
  const status = el('p', { className: 'cg-status', role: 'status', 'aria-live': 'polite' });
  try { state.users[0] = credentials(); } catch (error) { status.textContent = error.message; }

  function ref(anchor) {
    return `${root.dataset.reference}#${anchor}`;
  }
  function link(anchor, text = '配置说明') {
    return el('a', { href: ref(anchor), target: '_blank', rel: 'noopener' }, text);
  }
  function button(text, action, attrs = {}) {
    return el('button', { type: 'button', onclick: action, ...attrs }, text);
  }
  function bindError(key, control, container) {
    const message = el('span', { id: `cg-${key}-error`, className: 'cg-error' });
    control.id = `cg-${key}`;
    control.setAttribute('aria-describedby', `${control.id}-hint ${message.id}`);
    inputs.set(key, control);
    errorNodes.set(key, message);
    container.append(message);
  }
  function field(key, options = {}) {
    const definition = options.definition ?? INPUT.fields[key];
    if (!active(definition, options.scope ?? state, state)) return null;
    const { label, placeholder, hint, anchor, type = 'text', numeric = false } = definition.ui;
    const read = options.read ?? (() => state[key]);
    const write = options.write ?? (value => { state[key] = value; });
    const control = el('input', {
      type, placeholder, autocomplete: 'off', spellcheck: 'false',
      ...(numeric ? { inputmode: 'numeric' } : {}),
      oninput: () => { write(control.value); update(); },
    });
    control.value = read();
    const container = el('div', { className: 'cg-field' },
      el('label', { for: `cg-${key}` }, label), control,
      el('span', { id: `cg-${key}-hint`, className: 'cg-hint' }, hint, ' ', link(anchor)));
    bindError(key, control, container);
    return container;
  }
  function select(key, label, choices, onChange, anchor = 'transport') {
    const control = el('select', { onchange: () => {
      state[key] = key === 'activeUser' ? Number(control.value) : control.value;
      if (onChange) onChange();
      update();
    } });
    for (const entry of choices) {
      const [value, text] = Array.isArray(entry) ? entry : [entry, entry];
      control.append(el('option', { value }, text));
    }
    control.value = String(state[key]);
    const container = el('div', { className: 'cg-field' }, el('label', { for: `cg-${key}` }, label), control,
      el('span', { id: `cg-${key}-hint`, className: 'cg-hint' }, link(anchor)));
    bindError(key, control, container);
    return container;
  }
  function toggle(key, label, hint, rebuild = false) {
    const control = el('input', { type: 'checkbox', onchange: () => {
      state[key] = control.checked;
      if (rebuild) renderFields(key);
      update();
    } });
    control.checked = state[key];
    const container = el('div', { className: 'cg-toggle' }, el('label', { for: `cg-${key}` }, control, ' ', label),
      el('span', { id: `cg-${key}-hint`, className: 'cg-hint' }, hint));
    bindError(key, control, container);
    return container;
  }
  function section(number, title, ...children) {
    return el('section', { className: 'cg-section' }, el('h2', {}, el('span', { className: 'cg-step' }, number), title), ...children);
  }
  function grid(...children) { return el('div', { className: 'cg-grid' }, ...children); }

  const form = el('form', { className: 'cg-form', autocomplete: 'off', onsubmit: e => e.preventDefault() });
  const modeBar = el('div', { className: 'cg-modes', role: 'group', 'aria-label': '生成模式' });
  MODES.forEach(([mode, label]) => modeBar.append(button(label, () => {
    state.mode = mode;
    outputSide = mode === 'client' ? 'client' : 'server';
    renderFields(); update();
  }, { 'data-mode': mode })));
  const sideBar = el('div', { className: 'cg-sides', role: 'group', 'aria-label': '配置预览类型' });
  for (const side of ['server', 'client']) sideBar.append(button(side === 'server' ? '服务端' : '客户端', () => {
    outputSide = side; update();
  }, { 'data-side': side }));
  const formatSelect = el('select', { id: 'cg-format', 'aria-label': '输出格式', onchange: () => {
    state.format = formatSelect.value; update();
  } }, FORMATS.map(f => el('option', { value: f }, f.toUpperCase())));
  const preview = el('code', { id: 'cg-preview-code' });
  const filename = el('strong');
  const validity = el('span', { className: 'cg-validity', role: 'status' });
  const errorSummary = el('div', { className: 'cg-errors' });
  const notes = el('div', { className: 'cg-notes' });
  const command = el('code', { className: 'cg-command' });
  const copyButton = button('复制配置', async () => {
    if (!currentText) return;
    try { await navigator.clipboard.writeText(currentText); status.textContent = '已复制完整配置（含明文凭据）。'; }
    catch { status.textContent = '浏览器未允许复制，请使用下载配置。'; }
  }, { className: 'cg-secondary' });
  const downloadButton = button('下载配置', () => {
    if (!currentText) return;
    const blob = new Blob([currentText], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = el('a', { href: url, download: `${outputSide}.${state.format}` });
    root.append(a); a.click(); a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    status.textContent = `已下载 ${outputSide}.${state.format}（含明文凭据）。`;
  }, { className: 'cg-primary' });
  const reveal = el('input', { type: 'checkbox', id: 'cg-reveal', onchange: () => { showSecrets = reveal.checked; update(); } });
  const panel = el('aside', { className: 'cg-output', 'aria-label': '生成结果' },
    el('div', { className: 'cg-output-top' }, el('span', {}, '配置预览'), validity),
    el('div', { className: 'cg-toolbar' }, sideBar, formatSelect),
    el('div', { className: 'cg-filebar' }, filename, el('label', { for: 'cg-reveal' }, reveal, ' 显示密码')),
    errorSummary, el('pre', { className: 'cg-code', tabindex: '0', 'aria-label': '配置内容' }, preview),
    el('div', { className: 'cg-actions' }, copyButton, downloadButton),
    el('p', { className: 'cg-hint' }, '复制和下载始终包含真实密码。配对模式请分别保存两份配置。'),
    command, notes);
  root.replaceChildren(el('div', { className: 'cg-heading' },
    el('p', { className: 'cg-eyebrow' }, `TUIC ${VERSION} · QUINN`),
    el('h1', { id: 'config-generator-title' }, '配置文件生成器'),
    el('p', {}, '填写连接信息，生成相互匹配的服务端与客户端配置。'),
    el('p', { className: 'cg-privacy' }, '在浏览器本地处理 · 不保存输入 · 不上传凭据')), modeBar,
    el('div', { className: 'cg-workspace' }, form, panel), status);

  function renderFields(focusKey) {
    const advancedOpen = form.querySelector('details')?.open ?? false;
    inputs.clear(); errorNodes.clear(); form.replaceChildren();
    const address = [];
    if (hasClient(state)) address.push(grid(field('host'), field('port')));
    if (hasServer(state)) address.push(field('listen'));
    form.append(section('01', '连接信息', ...address));

    const users = el('div', { className: 'cg-users' });
    state.users.forEach((user, i) => {
      if (!hasServer(state) && i !== state.activeUser) return;
      const row = el('div', { className: 'cg-user' },
        el('div', { className: 'cg-row-title' }, el('strong', {}, `用户 ${i + 1}`),
          button('生成凭据', () => {
            try { state.users[i] = credentials(); renderFields(`users.${i}.uuid`); update(); status.textContent = `已更新用户 ${i + 1} 的 UUID 和密码。`; }
            catch (e) { status.textContent = e.message; }
          }),
          hasServer(state) && state.users.length > 1 ? button('移除', () => {
            state.users.splice(i, 1);
            if (state.activeUser === i) state.activeUser = 0;
            else if (state.activeUser > i) state.activeUser--;
            renderFields(); update();
          }, { 'aria-label': `移除用户 ${i + 1}` }) : null),
        field(`users.${i}.uuid`, { definition: USER.fields.uuid, scope: user, read: () => user.uuid, write: v => { user.uuid = v; } }),
        field(`users.${i}.password`, { definition: USER.fields.password, scope: user, read: () => user.password, write: v => { user.password = v; } }));
      users.append(row);
    });
    if (hasServer(state)) users.append(button('＋ 添加用户', () => {
      try { state.users.push(credentials()); } catch { state.users.push({ uuid: '', password: '' }); }
      renderFields(`users.${state.users.length - 1}.uuid`); update();
    }));
    if (hasClient(state) && state.users.length > 1) users.append(select('activeUser', '客户端使用的用户', state.users.map((_, i) => [i, `用户 ${i + 1}`]), () => renderFields('activeUser'), 'users'));
    form.append(section('02', '用户认证', users));

    const tls = [];
    if (hasServer(state)) {
      tls.push(select('tlsMode', '服务端证书方式', TLS_MODES, () => {
        // Never carry an insecure test setting into a new certificate mode.
        state.insecure = false; renderFields('tlsMode');
      }, 'tls'), field('hostname'));
      tls.push(field('certificate'), field('privateKey'), field('email'), field('dataDir'));
      if (state.tlsMode === 'acme') tls.push(el('p', { className: 'cg-callout' }, '域名需指向服务器，ACME HTTP-01 验证使用 TCP 80；同时开放 TUIC 的 UDP 监听端口。'));
      if (state.tlsMode === 'self') tls.push(el('p', { className: 'cg-callout' }, '仅用于受控测试。此版本的客户端不使用 tls.certificates；配对测试需明确跳过证书校验。'));
    }
    if (hasClient(state)) tls.push(field('sni'), toggle('insecure', '跳过证书校验（仅测试）', '会失去服务端身份验证；默认关闭。'));
    form.append(section('03', 'TLS 与证书', ...tls));
    if (hasClient(state)) form.append(section('04', '本地代理', field('local'),
      toggle('localAuth', '启用 SOCKS5 用户名和密码', '监听非本机地址时建议设置认证。', true),
      ...(state.localAuth ? [grid(field('localUsername'), field('localPassword'))] : [])));

    const advanced = el('details', { className: 'cg-section cg-advanced' }, el('summary', {}, '高级选项', el('span', {}, '日志 · 重连 · 端口转发')));
    advanced.open = advancedOpen;
    advanced.append(select('logLevel', '日志级别', LOG_LEVELS),
      toggle('zeroRtt', '启用 0-RTT', '默认关闭；早期数据可能被重放。配对模式会同时设置两端。'));
    if (hasServer(state)) advanced.append(select('controller', '服务端拥塞控制', CONTROLLERS));
    if (hasClient(state)) {
      advanced.append(toggle('lazy', '按需连接', '默认开启，首次代理请求时才连接服务端。'),
        toggle('reconnect', '断线自动重连', '默认开启，重试间隔按退避策略增加。', true));
      if (state.reconnect) advanced.append(grid(field('initialBackoff'), field('maxBackoff')));
      advanced.append(el('h3', {}, 'TCP / UDP 端口转发'));
      state.forwards.forEach((forward, i) => {
        const protocol = el('select', { id: `cg-forwards.${i}.protocol`, 'aria-label': `转发 ${i + 1} 协议`, onchange: () => {
          forward.protocol = protocol.value; renderFields(`forwards.${i}.listen`); update();
        } }, FORWARD.fields.protocol.choices.map(([value, label]) => el('option', { value }, label)));
        protocol.value = forward.protocol;
        const row = el('div', { className: 'cg-user' }, el('div', { className: 'cg-row-title' }, el('strong', {}, `转发 ${i + 1}`), protocol,
          button('移除', () => { state.forwards.splice(i, 1); renderFields(); update(); }, { 'aria-label': `移除转发 ${i + 1}` })),
          ...['listen', 'remote'].map(k => field(`forwards.${i}.${k}`, {
            definition: FORWARD.fields[k], scope: forward,
            read: () => forward[k], write: value => { forward[k] = value; },
          })));
        const timeout = field(`forwards.${i}.timeout`, { definition: FORWARD.fields.timeout, scope: forward,
          read: () => forward.timeout, write: value => { forward.timeout = value; } });
        if (timeout) row.append(timeout);
        advanced.append(row);
      });
      advanced.append(button('＋ 添加转发', () => {
        state.forwards.push(initialForward());
        renderFields(`forwards.${state.forwards.length - 1}.listen`); update();
      }), el('p', { className: 'cg-hint' }, '服务端默认拦截回环和私有目标地址。 ', link('forwarding')));
    }
    advanced.append(el('p', { className: 'cg-hint' }, '客户端 UDP 模式、自定义信任证书和拥塞控制暂未接入运行逻辑，本生成器不输出这些选项。 ', link('limitations', '查看版本限制')));
    form.append(advanced);
    if (focusKey) {
      const control = inputs.get(focusKey);
      if (control) { const details = control.closest('details'); if (details) details.open = true; control.focus(); }
    }
  }
  function update() {
    const errors = validate(state);
    const count = Object.keys(errors).length;
    for (const [key, control] of inputs) {
      control.setAttribute('aria-invalid', String(Boolean(errors[key])));
      errorNodes.get(key).textContent = errors[key] ?? '';
    }
    for (const b of modeBar.children) b.setAttribute('aria-pressed', String(b.dataset.mode === state.mode));
    for (const b of sideBar.children) {
      b.hidden = state.mode !== 'pair' && b.dataset.side !== state.mode;
      b.setAttribute('aria-pressed', String(b.dataset.side === outputSide));
    }
    filename.textContent = `${outputSide}.${state.format}`;
    command.textContent = `tuic-${outputSide} -c ${outputSide}.${state.format}`;
    validity.textContent = count ? `${count} 项待填写或修正` : '可导出';
    validity.dataset.valid = String(!count);
    errorSummary.replaceChildren();
    errorSummary.hidden = !count;
    if (count) {
      errorSummary.append(el('p', {}, '完成以下字段后即可生成：'));
      const list = el('ul');
      for (const [key, message] of Object.entries(errors)) list.append(el('li', {}, button(message, () => {
        const control = inputs.get(key); if (!control) return;
        const details = control.closest('details'); if (details) details.open = true;
        control.focus();
      })));
      errorSummary.append(list);
    }
    currentText = '';
    if (!count) {
      const config = buildConfigs(state)[outputSide];
      currentText = serialize(config, state.format);
      const display = showSecrets ? config : redactConfig(outputSide, config);
      preview.textContent = serialize(display, state.format);
    } else preview.textContent = '填写左侧配置，预览将在校验通过后显示。';
    copyButton.disabled = downloadButton.disabled = !!count;
    notes.replaceChildren();
    if (state.insecure && hasClient(state)) notes.append(el('p', { className: 'cg-callout' }, '当前客户端配置跳过证书校验，仅适合受控测试。'));
    const local = endpoint(state.local, true);
    if (hasClient(state) && local && !['127.0.0.1', '::1'].includes(local.host) && !state.localAuth) notes.append(el('p', { className: 'cg-callout' }, 'SOCKS5 正在监听非回环地址且未启用认证，请确认访问范围。'));
    if (state.zeroRtt) notes.append(el('p', { className: 'cg-callout' }, '0-RTT 早期数据可能被重放。'));
    notes.append(el('p', { className: 'cg-hint' }, '此处仅检查配置字段。证书文件、DNS、防火墙和实际连接需要在部署环境验证。'));
  }
  renderFields(); update();
}

if (typeof document !== 'undefined') {
  const start = () => initialize(document.getElementById('config-generator'));
  if (typeof document$ !== 'undefined') document$.subscribe(start);
  else if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, { once: true });
  else start();
}
