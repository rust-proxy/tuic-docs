// Deliberately limited to the JSON-compatible types produced by model.mjs.
// All strings/keys are quoted, never interpolated as configuration syntax.
const object = v => v !== null && typeof v === 'object' && !Array.isArray(v);
const quote = v => JSON.stringify(v).replace(/[\u007f\u0085\u2028\u2029]/g, c => `\\u${c.charCodeAt(0).toString(16).padStart(4, '0')}`);

function tomlScalar(v) {
  if (typeof v === 'string') {
    // JSON's \f escape is not accepted by TOML.
    return quote(v).replace(/\\(?:["\\/bfnrt]|u[\da-fA-F]{4})/g, escape => escape === '\\f' ? '\\u000c' : escape);
  }
  if (typeof v === 'boolean' || (typeof v === 'number' && Number.isFinite(v))) return String(v);
  if (Array.isArray(v)) return `[${v.map(tomlScalar).join(', ')}]`;
  throw new Error('不支持的 TOML 数据类型。');
}
function toml(config) {
  const lines = [];
  function table(data, path = [], array = false) {
    if (path.length) {
      if (lines.length) lines.push('');
      const name = path.map(quote).join('.');
      lines.push(array ? `[[${name}]]` : `[${name}]`);
    }
    const children = [];
    for (const [key, value] of Object.entries(data)) {
      if (object(value) || (Array.isArray(value) && value.some(object))) children.push([key, value]);
      else lines.push(`${quote(key)} = ${tomlScalar(value)}`);
    }
    for (const [key, value] of children) {
      if (Array.isArray(value)) value.forEach(item => table(item, [...path, key], true));
      else table(value, [...path, key]);
    }
  }
  table(config);
  return lines.join('\n') + '\n';
}
function yaml(value, depth = 0) {
  const pad = '  '.repeat(depth);
  if (Array.isArray(value)) {
    return value.map(item => object(item) ? `${pad}-\n${yaml(item, depth + 1)}` : `${pad}- ${quote(item)}\n`).join('');
  }
  return Object.entries(value).map(([key, item]) => {
    if (object(item) && Object.keys(item).length) return `${pad}${quote(key)}:\n${yaml(item, depth + 1)}`;
    if (Array.isArray(item) && item.length) return `${pad}${quote(key)}:\n${yaml(item, depth + 1)}`;
    return `${pad}${quote(key)}: ${quote(item)}\n`;
  }).join('');
}
export function serialize(config, format) {
  if (format === 'toml') return toml(config);
  if (format === 'yaml') return yaml(config);
  if (format === 'json') return JSON.stringify(config, null, 2) + '\n';
  throw new Error('不支持的输出格式。');
}
