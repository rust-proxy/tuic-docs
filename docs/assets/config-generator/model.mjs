import { OUTPUT } from './schema.mjs';
import { project, redact } from './dsl.mjs';
import { validate } from './validation.mjs';

export function buildConfigs(state) {
  if (Object.keys(validate(state)).length) throw new Error('配置尚未通过校验。');
  return project(OUTPUT, state);
}

export function redactConfig(side, config) {
  if (!Object.hasOwn(OUTPUT.fields, side)) throw new Error('不支持的配置类型。');
  return redact(OUTPUT.fields[side], config);
}
