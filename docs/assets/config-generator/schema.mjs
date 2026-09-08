import { string, boolean, integer, choice, object, list, record, defaults } from './dsl.mjs';
import { hostValue, ipKind, isDomain, endpoint, validPort } from './addresses.mjs';

// Baseline: Itsusinn/tuic 4719113, Wind 9025349. See tools/config-generator-reference.md.
export const VERSION = '2.0.0-dev4';
export const hasServer = s => s.mode !== 'client';
export const hasClient = s => s.mode !== 'server';
const certificate = s => hasServer(s) && s.tlsMode === 'certificate';
const acme = s => hasServer(s) && s.tlsMode === 'acme';
const localAuth = s => hasClient(s) && s.localAuth;
const reconnect = s => hasClient(s) && s.reconnect;
const required = value => value.trim() ? '' : '请填写此项。';
const positive = unit => value => /^\d+$/.test(value) && Number.isSafeInteger(Number(value)) && Number(value) > 0
  ? '' : `请输入大于 0 的安全整数（${unit}）。`;
const socksCredential = value => new TextEncoder().encode(value).length > 255 ? 'SOCKS5 认证字段最多 255 字节。' : required(value);
const ui = (label, placeholder, hint, anchor, extra = {}) => ({ label, placeholder, hint, anchor, ...extra });

export const USER = object({
  uuid: string({ check: value => {
    const id = value.trim().toLowerCase();
    return /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/.test(id) && !/^0{8}-(?:0{4}-){3}0{12}$/.test(id)
      ? '' : '请输入非全零的标准 UUID，或点击生成凭据。';
  }, ui: ui('UUID', '', '配对配置自动使用相同的 UUID。', 'users') }),
  password: string({ secret: true, check: value => value.length ? '' : '请输入密码，或点击生成凭据。',
    ui: ui('密码', '', '可填写已有密码，或重新生成随机密码。', 'users', { type: 'password' }) }),
}, { when: (user, state) => hasServer(state) || state.users[state.activeUser] === user });
export const FORWARD = object({
  protocol: choice([['tcp', 'TCP'], ['udp', 'UDP']]),
  listen: string({ check: v => endpoint(v, true) ? '' : '监听地址必须是 IP:端口。',
    ui: ui('本地监听地址', '127.0.0.1:8080', '', 'forwarding') }),
  remote: string({ check: v => endpoint(v) ? '' : '目标必须是域名或 IP 加端口。',
    ui: ui('远端目标地址', 'example.com:80', '', 'forwarding') }),
  timeout: string({ default: '60', when: f => f.protocol === 'udp', check: positive('秒'),
    ui: ui('会话超时（秒）', '', '', 'forwarding', { numeric: true }) }),
});

// Form schema: seeds, field constraints, visibility and presentation metadata.
export const INPUT = object({
  mode: choice([['pair', '配对生成'], ['server', '仅服务端'], ['client', '仅客户端']]),
  format: choice(['toml', 'json', 'yaml']),
  host: string({ when: hasClient, check: value => {
    const h = hostValue(value);
    return (ipKind(h) || isDomain(h)) && !['0.0.0.0', '::'].includes(h) ? '' : '请输入可连接的域名或 IP，不含端口、路径或通配监听地址。';
  }, ui: ui('连接域名或 IP', 'tuic.example.com', '客户端实际连接的主机，不含协议、路径或端口。', 'addresses') }),
  port: string({ default: '8443', when: hasClient, check: v => validPort(v) ? '' : '端口必须是 1–65535 的整数。',
    ui: ui('连接端口', '8443', '客户端连接的 UDP 端口；端口映射时可与监听端口不同。', 'addresses', { numeric: true }) }),
  listen: string({ default: '[::]:8443', when: hasServer, check: v => endpoint(v, true) ? '' : '请输入有效的 IP:端口，例如 [::]:8443。',
    ui: ui('服务端监听地址', '[::]:8443', '必须是 IP:端口；IPv6 使用方括号。', 'addresses') }),
  users: list(USER, { default: [defaults(USER)] }),
  activeUser: integer(),
  tlsMode: choice([['certificate', '已有受信任证书'], ['acme', 'ACME 自动证书'], ['self', '自签名测试']], { when: hasServer }),
  hostname: string({ when: hasServer,
    ui: ui('证书域名', 'tuic.example.com', '留空时采用连接域名；单独生成服务端时必填。', 'tls') }),
  certificate: string({ default: '/etc/tuic/fullchain.pem', when: certificate, check: required,
    ui: ui('服务端证书路径', '/etc/tuic/fullchain.pem', '部署机器上的证书链文件路径。', 'tls') }),
  privateKey: string({ default: '/etc/tuic/privatekey.pem', when: certificate, check: required,
    ui: ui('服务端私钥路径', '/etc/tuic/privatekey.pem', '只填写路径，不要粘贴私钥内容。', 'tls') }),
  email: string({ when: acme, check: v => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v) ? '' : '请填写有效的邮箱地址。',
    ui: ui('ACME 联系邮箱', 'admin@example.com', '用于证书申请。', 'tls', { type: 'email' }) }),
  dataDir: string({ default: '/var/lib/tuic', when: acme, check: required,
    ui: ui('服务端数据目录', '/var/lib/tuic', 'ACME 缓存目录需要可写，并在容器中持久化。', 'tls') }),
  insecure: boolean({ when: hasClient }),
  sni: string({ when: hasClient,
    ui: ui('客户端 SNI', 'tuic.example.com', '留空时使用证书域名或连接域名；连接 IP 时需填写。', 'tls') }),
  local: string({ default: '127.0.0.1:1080', when: hasClient, check: v => endpoint(v, true) ? '' : '请输入有效的 IP:端口，例如 127.0.0.1:1080。',
    ui: ui('本地 SOCKS5 监听地址', '127.0.0.1:1080', '默认只接受本机连接；对外监听时建议设置认证。', 'local') }),
  localAuth: boolean({ when: hasClient }),
  localUsername: string({ when: localAuth, check: socksCredential,
    ui: ui('SOCKS5 用户名', '', '与 TUIC 服务端认证相互独立。', 'local') }),
  localPassword: string({ when: localAuth, check: socksCredential, secret: true,
    ui: ui('SOCKS5 密码', '', '与用户名同时设置。', 'local', { type: 'password' }) }),
  controller: choice([['bbr', 'BBR'], ['bbr3', 'BBR3（当前与 BBR 共用实现）'], ['cubic', 'CUBIC'], ['newreno', 'New Reno']], { when: hasServer }),
  logLevel: choice(['trace', 'debug', 'info', 'warn', 'error', 'off'], { default: 'info' }),
  reconnect: boolean({ default: true, when: hasClient }),
  lazy: boolean({ default: true, when: hasClient }),
  initialBackoff: string({ default: '500', when: reconnect, check: positive('毫秒'),
    ui: ui('首次重连等待（毫秒）', '500', '首次重连前等待的时间。', 'transport', { numeric: true }) }),
  maxBackoff: string({ default: '30000', when: reconnect, check: positive('毫秒'),
    ui: ui('最大重连等待（毫秒）', '30000', '应大于或等于首次等待时间。', 'transport', { numeric: true }) }),
  zeroRtt: boolean(),
  forwards: list(FORWARD, { when: hasClient }),
});

// Export choices from the same definitions used by validation.
export const MODES = INPUT.fields.mode.choices;
export const FORMATS = INPUT.fields.format.choices;
export const TLS_MODES = INPUT.fields.tlsMode.choices;
export const CONTROLLERS = INPUT.fields.controller.choices;
export const LOG_LEVELS = INPUT.fields.logLevel.choices;
export const initialState = () => defaults(INPUT);
export const initialForward = () => defaults(FORWARD);

const trim = key => string({ from: s => s[key].trim() });
const alpn = () => list(string(), { value: ['h3'] });
const selectedUser = s => s.users[s.activeUser];
const forwardList = protocol => list(object({
  listen: trim('listen'), remote: trim('remote'),
  timeout: string({ from: f => `${Number(f.timeout)}s`, when: () => protocol === 'udp' }),
}), { from: s => s.forwards.filter(f => f.protocol === protocol), when: s => s.forwards.some(f => f.protocol === protocol) });

// Output schema: keys are the actual configuration keys, not form field names.
export const OUTPUT = object({
  server: object({
    server: trim('listen'),
    log_level: choice(LOG_LEVELS, { from: 'logLevel' }),
    zero_rtt_handshake: boolean({ value: true, when: s => s.zeroRtt }),
    data_dir: string({ from: s => s.dataDir.trim(), when: acme }),
    users: record(string({ secret: true }), { from: s => Object.fromEntries(s.users.map(u => [u.uuid.trim().toLowerCase(), u.password])) }),
    tls: object({
      hostname: string({ from: s => s.hostname.trim() || hostValue(s.host) }),
      alpn: alpn(),
      certificate: string({ from: s => s.certificate.trim(), when: certificate }),
      private_key: string({ from: s => s.privateKey.trim(), when: certificate }),
      auto_ssl: boolean({ value: true, when: acme }),
      acme_email: string({ from: s => s.email.trim(), when: acme }),
      self_sign: boolean({ value: true, when: s => s.tlsMode === 'self' }),
    }),
    backend: object({
      mode: choice(['quinn'], { value: 'quinn' }),
      quinn: object({ congestion_control: object({ controller: choice(CONTROLLERS, { from: 'controller' }) }) }),
    }),
  }, { when: hasServer }),
  client: object({
    server: string({ from: s => {
      const host = hostValue(s.host);
      return `${ipKind(host) === 6 ? `[${host}]` : host}:${Number(s.port)}`;
    } }),
    // Explicit IP bypasses this baseline's lookup_host formatting for IPv6 literals.
    ip: string({ from: s => hostValue(s.host), when: s => Boolean(ipKind(hostValue(s.host))) }),
    uuid: string({ from: s => selectedUser(s).uuid.trim().toLowerCase() }),
    password: string({ from: s => selectedUser(s).password, secret: true }),
    log_level: choice(LOG_LEVELS, { from: 'logLevel' }),
    zero_rtt_handshake: boolean({ value: true, when: s => s.zeroRtt }),
    reconnect: boolean({ from: 'reconnect' }),
    lazy: boolean({ from: 'lazy' }),
    reconnect_initial_backoff: string({ from: s => `${Number(s.initialBackoff)}ms`, when: reconnect }),
    reconnect_max_backoff: string({ from: s => `${Number(s.maxBackoff)}ms`, when: reconnect }),
    tls: object({
      sni: string({ from: s => s.sni.trim() || (hasServer(s) ? s.hostname.trim() : '') || hostValue(s.host) }),
      alpn: alpn(),
      skip_cert_verify: boolean({ from: 'insecure' }),
    }),
    local: object({
      server: trim('local'),
      username: string({ from: 'localUsername', when: localAuth }),
      password: string({ from: 'localPassword', when: localAuth, secret: true }),
      tcp_forward: forwardList('tcp'),
      udp_forward: forwardList('udp'),
    }),
  }, { when: hasClient }),
});

export function credentials(cryptoApi = globalThis.crypto) {
  if (!cryptoApi?.getRandomValues || !cryptoApi?.randomUUID) {
    throw new Error('当前浏览器无法安全生成凭据，请使用 HTTPS 或 localhost，或手动填写。');
  }
  const bytes = cryptoApi.getRandomValues(new Uint8Array(24));
  return { uuid: cryptoApi.randomUUID(), password: Array.from(bytes, b => b.toString(16).padStart(2, '0')).join('') };
}
