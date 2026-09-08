import test from 'node:test';
import assert from 'node:assert/strict';
import { string, boolean, integer, choice, object, list, record, defaults, active, validateSchema, project, redact, ConfigDslError } from '../../docs/assets/config-generator/dsl.mjs';

test('declarations reject misspelled options, ambiguous mappings and invalid enums', () => {
  assert.throws(() => string({ defaut: '' }), /defaut/);
  assert.throws(() => string({ from: 'name', value: 'fixed' }), /from/);
  assert.throws(() => string({ from: 1 }), /from/);
  assert.throws(() => string({ when: true }), /when/);
  assert.throws(() => string({ check: 'required' }), /check/);
  assert.throws(() => choice([]), /枚举/);
  assert.throws(() => choice(['a', 'a']), /枚举/);
  assert.throws(() => choice([1]), /枚举/);
  assert.throws(() => object({ field: {} }), /类型/);
});

test('defaults seed every branch and clone collections without running callbacks', () => {
  const schema = object({
    name: string(), enabled: boolean(), count: integer(), mode: choice([['a', 'A'], ['b', 'B']]),
    nested: object({ port: string({ default: '8443' }) }, { when: () => false }),
    rows: list(object({ tags: list(string()) }), { default: [{ tags: ['one'] }] }),
    map: record(string()),
    derived: string({ from: () => assert.fail('defaults must not evaluate mappings') }),
  });
  const first = defaults(schema);
  assert.deepEqual(first, { name: '', enabled: false, count: 0, mode: 'a', nested: { port: '8443' }, rows: [{ tags: ['one'] }], map: {}, derived: '' });
  first.rows[0].tags.push('two');
  assert.deepEqual(defaults(schema).rows, [{ tags: ['one'] }]);
});

test('validation reports nested paths and never coerces types or applies defaults', () => {
  const schema = object({
    enabled: boolean({ default: false }), count: integer(), mode: choice(['a', 'b']),
    rows: list(object({ name: string({ check: value => value ? '' : 'required' }) })),
    secrets: record(string({ secret: true })),
  });
  const errors = validateSchema(schema, { count: 1.5, mode: 'c', rows: [{ name: '' }, { name: null }], secrets: { token: false } });
  assert.deepEqual(Object.keys(errors), ['enabled', 'count', 'mode', 'rows.0.name', 'rows.1.name', 'secrets.token']);
  for (const bad of [null, [], 'text']) assert.ok(validateSchema(schema, bad).$);
  assert.ok(validateSchema(list(string()), Array(1))['0']);
  assert.ok(validateSchema(integer(), Number.MAX_SAFE_INTEGER + 1).$);
});

test('conditional validation sees the item and root and skips inactive requirements', () => {
  const item = object({
    enabled: boolean(),
    name: string({ when: (row, root) => row.enabled && root.enabled, check: value => value ? '' : 'required' }),
  });
  const schema = object({ enabled: boolean(), rows: list(item) });
  const data = { enabled: true, rows: [{ enabled: false, name: '' }, { enabled: true, name: '' }] };
  assert.deepEqual(validateSchema(schema, data), { 'rows.1.name': 'required' });
  data.enabled = false;
  assert.deepEqual(validateSchema(schema, data), {});
  assert.equal(active(item.fields.name, data.rows[1], data), false);
  data.rows[0].name = null;
  assert.ok(validateSchema(schema, data, { typesOnly: true })['rows.0.name']);
});

test('projection maps nested scopes, constants, record values and typed list items', () => {
  const schema = object({
    service: object({
      title: string({ from: 'name' }),
      enabled: boolean({ value: false }),
      ports: list(integer(), { from: row => row.ports.map(Number) }),
      routes: list(object({
        target: string({ from: (row, root) => `${root.prefix}/${row.name}` }),
      }), { from: 'routes' }),
    }, { from: 'settings' }),
    passwords: record(string({ secret: true }), { from: 'passwords' }),
    alpn: list(string(), { value: ['h3'] }),
  });
  const state = { prefix: 'demo', settings: { name: 'proxy', ports: ['80', '443'], routes: [{ name: 'one' }] }, passwords: { alice: ' password ' } };
  const config = project(schema, state);
  assert.deepEqual(config, { service: { title: 'proxy', enabled: false, ports: [80, 443], routes: [{ target: 'demo/one' }] }, passwords: { alice: ' password ' }, alpn: ['h3'] });
  config.alpn.push('test');
  assert.deepEqual(project(schema, state).alpn, ['h3']);
  assert.equal(state.passwords.alice, ' password ');
});

test('conditions omit keys before evaluating mappings and preserve false, zero and empty values', () => {
  const schema = object({
    hidden: string({ when: () => false, from: () => assert.fail('inactive mapping evaluated') }),
    flag: boolean({ value: false }), zero: integer({ value: 0 }), text: string({ value: '' }),
    empty: list(string(), { value: [] }),
  });
  assert.deepEqual(project(schema, {}), { flag: false, zero: 0, text: '', empty: [] });
  assert.equal(project(object({}, { when: () => false }), {}), undefined);
});

test('mapping failures include paths without including offending secret values', () => {
  const schema = object({ nested: object({ token: integer({ from: 'password', secret: true }) }) });
  assert.throws(() => project(schema, { password: 'do-not-log-this' }), error => {
    assert.ok(error instanceof ConfigDslError);
    assert.deepEqual(Object.keys(error.errors), ['nested.token']);
    assert.ok(!error.message.includes('do-not-log-this'));
    return true;
  });
  assert.throws(() => project(string({ from: 'missing', default: 'fallback' }), {}), ConfigDslError);
  assert.throws(() => project(list(string(), { value: [1] }), {}), ConfigDslError);
  assert.throws(() => project(choice(['a'], { value: 'b' }), {}), ConfigDslError);
});

test('redaction follows nested secret declarations and does not re-evaluate mappings', () => {
  const schema = object({
    public: string(),
    users: record(string({ secret: true })),
    rows: list(object({ password: string({ secret: true, from: () => assert.fail('mapping reran') }) })),
    absent: string({ secret: true, when: () => assert.fail('condition reran') }),
  });
  const config = { public: 'visible', users: { alice: 'one', bob: 'two' }, rows: [{ password: 'three' }] };
  const before = structuredClone(config);
  assert.deepEqual(redact(schema, config), { public: 'visible', users: { alice: '••••••••', bob: '••••••••' }, rows: [{ password: '••••••••' }] });
  assert.deepEqual(config, before);
  assert.throws(() => redact(schema, { surprise: 'hidden' }), ConfigDslError);
});

test('dynamic keys remain own data properties without prototype mutation', () => {
  const data = JSON.parse('{"__proto__":"secret","constructor":"secret"}');
  const schema = record(string({ secret: true }));
  const config = project(schema, data);
  assert.deepEqual(config, data);
  assert.equal(Object.getPrototypeOf(config), Object.prototype);
  assert.equal(redact(schema, config).__proto__, '••••••••');
  const errors = validateSchema(record(integer()), data);
  assert.ok(Object.hasOwn(errors, '__proto__'));
  assert.equal(Object.getPrototypeOf(errors), Object.prototype);
});
