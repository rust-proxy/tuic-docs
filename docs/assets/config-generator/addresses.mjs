export function ipKind(value) {
  if (/^(?:\d{1,3}\.){3}\d{1,3}$/.test(value) && value.split('.').every(n => Number(n) <= 255 && (n === '0' || !n.startsWith('0')))) return 4;
  if (!value.includes(':') || /[\s%/\[\]?#@]/.test(value)) return 0;
  try { return new URL(`http://[${value}]/`).hostname.startsWith('[') ? 6 : 0; } catch { return 0; }
}
export function hostValue(value) {
  const v = value.trim();
  return v.startsWith('[') && v.endsWith(']') ? v.slice(1, -1) : v;
}
export function isDomain(v) {
  return v.length <= 253 && !/^\d+(?:\.\d+)*$/.test(v) && v.split('.').every(label => /^[a-z\d](?:[a-z\d-]{0,61}[a-z\d])?$/i.test(label));
}
export const validPort = v => /^\d+$/.test(String(v)) && Number(v) >= 1 && Number(v) <= 65535;
export function endpoint(value, literal = false) {
  const m = value.trim().match(/^(?:\[([^\]]+)\]|([^:\s]+)):(\d+)$/);
  if (!m || !validPort(m[3])) return null;
  const host = m[1] ?? m[2];
  const kind = ipKind(host);
  if (m[1] ? kind !== 6 : (!kind && (literal || !isDomain(host)))) return null;
  return { host, port: Number(m[3]), kind };
}
