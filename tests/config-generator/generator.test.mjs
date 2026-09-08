import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { initialState, initialForward, credentials, FORMATS, CONTROLLERS } from '../../docs/assets/config-generator/schema.mjs';
import { validate, endpoint, ipKind } from '../../docs/assets/config-generator/validation.mjs';
import { buildConfigs, redactConfig } from '../../docs/assets/config-generator/model.mjs';
import { serialize } from '../../docs/assets/config-generator/serializers.mjs';

function ready() {
  const s = initialState();
  s.host = 'tuic.example.com';
  s.users = [credentials()];
  return s;
}

test('DSL preserves the complete default paired configuration', () => {
  const s = ready();
  const { uuid, password } = s.users[0];
  assert.deepEqual(buildConfigs(s), {
    server: {
      server: '[::]:8443', log_level: 'info', users: { [uuid]: password },
      tls: { hostname: 'tuic.example.com', alpn: ['h3'], certificate: '/etc/tuic/fullchain.pem', private_key: '/etc/tuic/privatekey.pem' },
      backend: { mode: 'quinn', quinn: { congestion_control: { controller: 'bbr' } } },
    },
    client: {
      server: 'tuic.example.com:8443', uuid, password, log_level: 'info', reconnect: true, lazy: true,
      reconnect_initial_backoff: '500ms', reconnect_max_backoff: '30000ms',
      tls: { sni: 'tuic.example.com', alpn: ['h3'], skip_cert_verify: false }, local: { server: '127.0.0.1:1080' },
    },
  });
  assert.deepEqual(initialForward(), { protocol: 'tcp', listen: '', remote: '', timeout: '60' });
  s.users[0].password = 'changed'; s.forwards.push(initialForward());
  assert.equal(initialState().users[0].password, '');
  assert.deepEqual(initialState().forwards, []);
});

test('DSL rejects malformed state without throwing or silently coercing it', () => {
  for (const s of [null, {}, { ...ready(), users: [null] }, { ...ready(), forwards: Array(1) }, { ...ready(), insecure: 'false' }]) {
    assert.ok(Object.keys(validate(s)).length);
    assert.throws(() => buildConfigs(s), /配置尚未通过校验/);
  }
});

test('DSL masks every emitted password without modifying exports or input', () => {
  const s = ready(); s.users.push(credentials()); s.localAuth = true; s.localUsername = 'tester'; s.localPassword = 'local secret';
  const config = buildConfigs(s);
  const original = structuredClone(config);
  assert.deepEqual(Object.values(redactConfig('server', config.server).users), ['••••••••', '••••••••']);
  const client = redactConfig('client', config.client);
  assert.equal(client.password, '••••••••'); assert.equal(client.local.password, '••••••••');
  assert.equal(client.local.username, 'tester'); assert.equal(client.uuid, s.users[0].uuid);
  assert.deepEqual(config, original);
});
test('empty state cannot export; random credentials use crypto', () => {
  assert.ok(validate(initialState()).host);
  assert.throws(() => buildConfigs(initialState()));
  const c = credentials();
  assert.match(c.uuid, /^[\da-f]{8}-[\da-f]{4}-4[\da-f]{3}-[89ab][\da-f]{3}-[\da-f]{12}$/);
  assert.equal(c.password.length, 48);
  assert.notEqual(credentials().uuid, c.uuid);
  assert.throws(() => credentials({}));
});
test('pair shares selected user and separates listen and connect endpoints', () => {
  const s = ready(); s.port = '443'; s.users.push(credentials()); s.activeUser = 1;
  const c = buildConfigs(s);
  assert.equal(c.server.server, '[::]:8443');
  assert.equal(c.client.server, 'tuic.example.com:443');
  assert.equal(c.server.users[c.client.uuid], c.client.password);
  assert.equal(c.client.uuid, s.users[1].uuid);
  assert.equal(c.client.tls.skip_cert_verify, false);
  assert.deepEqual(c.client.tls.alpn, ['h3']);
  assert.deepEqual(c.server.tls.alpn, c.client.tls.alpn);
  assert.equal(Object.keys(c.server.users).length, 2);
});
test('duplicate and nil UUIDs, empty passwords are rejected', () => {
  const s = ready(); s.users.push({ ...s.users[0], uuid: s.users[0].uuid.toUpperCase() });
  assert.ok(validate(s)['users.1.uuid']);
  s.users[1].uuid = ['0'.repeat(8), ...Array(3).fill('0'.repeat(4)), '0'.repeat(12)].join('-');
  assert.ok(validate(s)['users.1.uuid']);
  s.users[0].password = ''; assert.ok(validate(s)['users.0.password']);
});
test('server and client modes validate only relevant fields', () => {
  const s = ready(); s.mode = 'server'; s.host = ''; s.hostname = 'tuic.example.com'; s.local = 'bad';
  assert.deepEqual(Object.keys(buildConfigs(s)), ['server']);
  s.mode = 'client'; s.host = 'tuic.example.com'; s.local = '127.0.0.1:1080'; s.listen = 'bad'; s.tlsMode = 'self';
  s.certificate = ''; s.privateKey = '';
  assert.deepEqual(Object.keys(buildConfigs(s)), ['client']);
});

test('inactive DSL branches retain editable state without blocking generation', () => {
  const s = ready();
  Object.assign(s, { mode: 'client', controller: 'unsupported', tlsMode: 'unsupported', email: 'invalid',
    reconnect: false, initialBackoff: 'bad', maxBackoff: 'bad' });
  s.users.push({ uuid: '', password: '' });
  s.forwards.push({ ...initialForward(), listen: '127.0.0.1:8080', remote: 'example.com:80', timeout: 'bad' });
  assert.deepEqual(validate(s), {});
  const config = buildConfigs(s).client;
  assert.equal(config.local.tcp_forward[0].timeout, undefined);
  assert.equal(config.reconnect_initial_backoff, undefined);
  s.forwards[0].protocol = 'udp';
  assert.ok(validate(s)['forwards.0.timeout']);
  s.forwards[0].protocol = 'tcp'; s.activeUser = 1;
  assert.ok(validate(s)['users.1.uuid']);
  assert.ok(validate(s)['users.1.password']);
});
test('TLS modes omit stale keys and never silently disable verification', () => {
  const s = ready(); s.tlsMode = 'self';
  assert.ok(validate(s).insecure);
  s.insecure = true;
  let c = buildConfigs(s);
  assert.deepEqual(Object.keys(c.server.tls).sort(), ['alpn', 'hostname', 'self_sign']);
  s.tlsMode = 'acme'; s.insecure = false; s.email = 'admin@example.com';
  c = buildConfigs(s);
  assert.deepEqual(Object.keys(c.server.tls).sort(), ['acme_email', 'alpn', 'auto_ssl', 'hostname']);
  assert.equal(c.server.data_dir, s.dataDir);
  s.tlsMode = 'certificate'; c = buildConfigs(s);
  assert.equal(c.server.data_dir, undefined); assert.equal(c.server.tls.auto_ssl, undefined);
  assert.equal(c.client.tls.skip_cert_verify, false);
});
test('IPv6 is bracketed, uses explicit ip and requires DNS SNI', () => {
  const s = ready(); s.host = '[2001:db8::1]';
  assert.ok(validate(s).sni); s.hostname = 'tuic.example.com';
  const c = buildConfigs(s).client;
  assert.equal(c.server, '[2001:db8::1]:8443'); assert.equal(c.ip, '2001:db8::1');
  assert.equal(c.tls.sni, 'tuic.example.com');
  assert.equal(ipKind('0:0:0:0:0:0:0:1'), 6);
  assert.equal(ipKind('256.0.0.1'), 0);
  assert.equal(endpoint('::1:443'), null);
  assert.equal(endpoint('[1.2.3.4]:443'), null);
});
test('addresses and integer port boundaries reject ambiguous input', () => {
  for (const host of ['0.0.0.0', '::', 'https://example.com', 'example.com/path', 'host:443', '999.999.1.1', 'a b.com']) {
    const s = ready(); s.host = host; assert.ok(validate(s).host, host);
  }
  for (const port of ['0', '65536', '1.2', '-1', '1e3', '']) {
    const s = ready(); s.port = port; assert.ok(validate(s).port, port);
  }
  for (const port of ['1', '65535']) { const s = ready(); s.port = port; assert.deepEqual(validate(s), {}); }
});
test('reconnect bounds, SOCKS credentials, stale optional fields', () => {
  const s = ready(); s.maxBackoff = '100'; assert.ok(validate(s).maxBackoff);
  s.reconnect = false; s.localAuth = true;
  assert.ok(validate(s).localPassword); s.localUsername = 'u'; s.localPassword = '字'.repeat(86);
  assert.ok(validate(s).localPassword); s.localPassword = '字'.repeat(85);
  assert.deepEqual(validate(s), {}); s.localAuth = false;
  const c = buildConfigs(s).client;
  assert.equal(c.local.password, undefined); assert.equal(c.reconnect_initial_backoff, undefined);
});
test('TCP/UDP forwards remain arrays; duplicate bindings and SOCKS collision rejected', () => {
  const s = ready();
  s.forwards = [{ protocol: 'tcp', listen: '127.0.0.1:8080', remote: 'example.com:80', timeout: '60' },
    { protocol: 'udp', listen: '[::1]:8053', remote: '[2001:db8::53]:53', timeout: '45' }];
  const c = buildConfigs(s).client;
  assert.equal(c.local.tcp_forward[0].timeout, undefined);
  assert.equal(c.local.udp_forward[0].timeout, '45s');
  s.forwards.push({ ...s.forwards[1], listen: '[0:0:0:0:0:0:0:1]:8053' });
  assert.ok(validate(s)['forwards.2.listen']);
  s.forwards[2].listen = '127.0.0.1:1080'; assert.ok(validate(s)['forwards.2.listen']);
});
test('all offered controllers serialize enum spelling; ignored client fields omitted', () => {
  for (const [controller] of CONTROLLERS) {
    const s = ready(); s.controller = controller;
    const c = buildConfigs(s);
    assert.equal(c.server.backend.quinn.congestion_control.controller, controller);
    for (const key of ['backend', 'udp_relay_mode', 'proxy']) assert.equal(c.client[key], undefined);
    assert.equal(c.client.tls.certificates, undefined);
  }
});
test('independent parsers round-trip modes, nested arrays and hostile strings', () => {
  const cases = [];
  const special = 'quotes " \\f \\" \\u000c \f\n\r\t\b\0\x7f\x85\u2028\u2029 中文 😀 # ]\n[users]\n- yes: null';
  for (const mode of ['pair', 'server', 'client']) {
    for (const tlsMode of ['certificate', 'acme', 'self']) {
      const s = ready(); Object.assign(s, { mode, tlsMode, hostname: 'tuic.example.com', email: 'admin@example.com', insecure: tlsMode === 'self', localAuth: true, localUsername: 'tester', localPassword: special });
      s.users.push(credentials()); s.activeUser = 1; s.users[1].password = special;
      s.certificate = 'C:\\certs\\f "证书".pem';
      s.forwards = ['tcp', 'udp'].flatMap((protocol, p) => [0, 1].map(i => ({ protocol, listen: `127.0.0.1:${8100 + p * 100 + i}`, remote: '[2001:db8::53]:53', timeout: '60' })));
      for (const [side, config] of Object.entries(buildConfigs(s))) cases.push({
        name: `${mode}/${tlsMode}/${side}`, expected: config,
        formats: Object.fromEntries(FORMATS.map(format => [format, serialize(config, format)])),
      });
    }
  }
  const python = process.env.PYTHON ?? fileURLToPath(new URL('../../.venv/Scripts/python.exe', import.meta.url));
  const result = execFileSync(python, [fileURLToPath(new URL('./roundtrip.py', import.meta.url))], { input: JSON.stringify(cases), encoding: 'utf8' });
  assert.match(result, /12 objects round-tripped/);
});
