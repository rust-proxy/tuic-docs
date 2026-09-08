import { hasClient, hasServer, INPUT } from './schema.mjs';
import { validateSchema } from './dsl.mjs';
import { hostValue, isDomain, endpoint } from './addresses.mjs';
export { ipKind, hostValue, isDomain, validPort, endpoint } from './addresses.mjs';

export function validate(s) {
  // Check structure first so cross-field rules cannot dereference malformed input.
  const shapeErrors = validateSchema(INPUT, s, { typesOnly: true });
  if (Object.keys(shapeErrors).length) return shapeErrors;
  const errors = validateSchema(INPUT, s);
  if (hasClient(s)) {
    const sni = s.sni.trim() || (hasServer(s) ? s.hostname.trim() : '') || hostValue(s.host);
    if (!isDomain(sni)) errors.sni = '请填写证书对应的 DNS 域名作为 SNI。';
    if (!Number.isInteger(s.activeUser) || !s.users[s.activeUser]) errors.activeUser = '请选择一个用户。';
    if (s.reconnect && !errors.maxBackoff && !errors.initialBackoff && Number(s.maxBackoff) < Number(s.initialBackoff)) {
      errors.maxBackoff = '最大等待不能小于首次等待。';
    }
    const used = new Set();
    for (const [i, f] of s.forwards.entries()) {
      const key = `forwards.${i}`;
      const listen = endpoint(f.listen, true);
      if (listen) {
        // Canonicalize IPv6 for duplicate detection (e.g. ::1 and 0:0:0:0:0:0:0:1).
        const address = listen.kind === 6 ? new URL(`http://[${listen.host}]/`).hostname : listen.host;
        const id = `${f.protocol}:${address}:${listen.port}`;
        if (used.has(id)) errors[`${key}.listen`] = '同一协议的监听地址重复。';
        used.add(id);
        const socks = endpoint(s.local, true);
        if (socks && socks.port === listen.port) errors[`${key}.listen`] = '请使用与 SOCKS5 不同的监听端口。';
      }
    }
  }
  if (hasServer(s)) {
    const name = s.hostname.trim() || (hasClient(s) ? hostValue(s.host) : '');
    if (!isDomain(name)) errors.hostname = '请填写有效的证书域名。';
    if (s.tlsMode === 'acme' && !name.includes('.')) errors.hostname = 'ACME 需要公开域名。';
    if (s.tlsMode === 'self' && hasClient(s) && !s.insecure) errors.insecure = '自签名测试需要明确开启跳过证书校验，或改用受信任证书。';
  }
  const seen = new Set();
  if (!s.users.length) errors.users = '至少需要一个用户。';
  s.users.forEach((u, i) => {
    if (!hasServer(s) && i !== s.activeUser) return;
    const id = u.uuid.trim().toLowerCase();
    if (!errors[`users.${i}.uuid`] && seen.has(id)) errors[`users.${i}.uuid`] = 'UUID 重复，请为每个用户使用不同 UUID。';
    seen.add(id);
  });
  return errors;
}
