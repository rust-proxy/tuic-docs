// Config DSL v1: trusted, declarative ES modules; never evaluate user input as code.
const kinds = new Set(['string', 'boolean', 'integer', 'enum', 'object', 'list', 'record']);
const options = new Set(['default', 'from', 'value', 'when', 'check', 'secret', 'ui']);
const omitted = Symbol('omitted');
const isObject = value => value !== null && typeof value === 'object' && !Array.isArray(value)
  && [Object.prototype, null].includes(Object.getPrototypeOf(value));
const own = (value, key) => Object.hasOwn(value, key);
const pathKey = path => path.join('.') || '$';

function node(kind, settings = {}, structure = {}) {
  for (const key of Object.keys(settings)) if (!options.has(key)) throw new Error(`未知 DSL 选项：${key}`);
  if (own(settings, 'from') && own(settings, 'value')) throw new Error('from 与 value 不能同时使用。');
  if (own(settings, 'from') && typeof settings.from !== 'string' && typeof settings.from !== 'function') throw new Error('from 必须是字段名或函数。');
  for (const key of ['when', 'check']) if (own(settings, key) && typeof settings[key] !== 'function') throw new Error(`${key} 必须是函数。`);
  return Object.freeze({ kind, ...settings, ...structure });
}
function assertNode(schema) {
  if (!schema || !kinds.has(schema.kind)) throw new Error('无效的 DSL 类型。');
}
export const string = options => node('string', options);
export const boolean = options => node('boolean', options);
export const integer = options => node('integer', options);
export function choice(choices, options) {
  if (!Array.isArray(choices) || !choices.length) throw new Error('枚举至少需要一个选项。');
  const values = choices.map(entry => Array.isArray(entry) ? entry[0] : entry);
  if (values.some(value => typeof value !== 'string') || new Set(values).size !== values.length) throw new Error('枚举值必须是互不重复的字符串。');
  return node('enum', options, { choices, values });
}
export function object(fields, options) {
  Object.values(fields).forEach(assertNode);
  return node('object', options, { fields: Object.freeze({ ...fields }) });
}
export function list(item, options) {
  assertNode(item);
  return node('list', options, { item });
}
export function record(item, options) {
  assertNode(item);
  return node('record', options, { item });
}

// Defaults are form seeds, not fallback values for invalid/missing input.
export function defaults(schema) {
  if (own(schema, 'default')) return structuredClone(schema.default);
  if (schema.kind === 'object') return Object.fromEntries(Object.entries(schema.fields).map(([key, child]) => [key, defaults(child)]));
  if (schema.kind === 'list') return [];
  if (schema.kind === 'record') return {};
  if (schema.kind === 'enum') return schema.values[0];
  return { string: '', boolean: false, integer: 0 }[schema.kind];
}
export const active = (schema, scope, root = scope) => !schema.when || Boolean(schema.when(scope, root));

function typeError(schema, value) {
  switch (schema.kind) {
    case 'string': return typeof value === 'string' ? '' : '请输入文本。';
    case 'boolean': return typeof value === 'boolean' ? '' : '请选择布尔值。';
    case 'integer': return Number.isSafeInteger(value) ? '' : '请输入安全整数。';
    case 'enum': return schema.values.includes(value) ? '' : '请选择有效选项。';
    case 'list': return Array.isArray(value) ? '' : '必须是列表。';
    case 'object': case 'record': return isObject(value) ? '' : '必须是对象。';
    default: throw new Error('无效的 DSL 类型。');
  }
}

// Returns field paths, never the offending values (which may contain secrets).
export function validateSchema(schema, data, { typesOnly = false } = {}) {
  const errors = {};
  function visit(current, value, scope, path) {
    if (!typesOnly && !active(current, scope, data)) return;
    const typeMessage = typesOnly && current.kind === 'enum' ? (typeof value === 'string' ? '' : '请输入文本。') : typeError(current, value);
    const message = typeMessage || (!typesOnly && current.check?.(value, scope, data));
    if (message) { Object.defineProperty(errors, pathKey(path), { value: message, enumerable: true, configurable: true }); return; }
    if (current.kind === 'object') {
      for (const [key, child] of Object.entries(current.fields)) visit(child, own(value, key) ? value[key] : undefined, value, [...path, key]);
    } else if (current.kind === 'list') {
      for (const [index, item] of value.entries()) visit(current.item, item, item, [...path, index]);
    } else if (current.kind === 'record') {
      for (const [key, item] of Object.entries(value)) visit(current.item, item, item, [...path, key]);
    }
  }
  visit(schema, data, data, []);
  return errors;
}

export class ConfigDslError extends Error {
  constructor(errors) {
    super(`配置映射不符合 DSL：${Object.keys(errors).join(', ')}`);
    this.name = 'ConfigDslError';
    this.errors = errors;
  }
}

// Object blocks keep their scope; from changes it. Collections introduce item scopes.
export function project(schema, state) {
  const errors = {};
  function visit(current, scope, path) {
    if (!active(current, scope, state)) return omitted;
    const source = own(current, 'value') ? structuredClone(current.value)
      : typeof current.from === 'function' ? current.from(scope, state)
      : typeof current.from === 'string' ? (isObject(scope) && own(scope, current.from) ? scope[current.from] : undefined)
      : scope;
    let result = source;
    if (current.kind === 'object') {
      result = Object.fromEntries(Object.entries(current.fields).map(([key, child]) => [key, visit(child, source, [...path, key])])
        .filter(([, value]) => value !== omitted));
    } else if (current.kind === 'list' && Array.isArray(source)) {
      result = Array.from(source, (item, index) => visit(current.item, item, [...path, index])).filter(value => value !== omitted);
    } else if (current.kind === 'record' && isObject(source)) {
      result = Object.fromEntries(Object.entries(source).map(([key, item]) => [key, visit(current.item, item, [...path, key])])
        .filter(([, value]) => value !== omitted));
    }
    const message = typeError(current, result) || current.check?.(result, scope, state);
    if (message) Object.defineProperty(errors, pathKey(path), { value: message, enumerable: true, configurable: true });
    return result;
  }
  const result = visit(schema, state, []);
  if (Object.keys(errors).length) throw new ConfigDslError(errors);
  return result === omitted ? undefined : result;
}

// Walk the already projected shape, without re-evaluating input conditions/mappings.
export function redact(schema, data, mask = '••••••••') {
  if (schema.secret) return mask;
  if (schema.kind === 'object') return Object.fromEntries(Object.entries(data).map(([key, value]) => {
    if (!own(schema.fields, key)) throw new ConfigDslError({ [key]: '未声明的字段。' });
    return [key, redact(schema.fields[key], value, mask)];
  }));
  if (schema.kind === 'list') return data.map(value => redact(schema.item, value, mask));
  if (schema.kind === 'record') return Object.fromEntries(Object.entries(data).map(([key, value]) => [key, redact(schema.item, value, mask)]));
  return data;
}
