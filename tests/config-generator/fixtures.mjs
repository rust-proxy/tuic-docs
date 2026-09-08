// Runtime-only credentials; output must remain in the ignored .cache directory.
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { initialState, credentials, FORMATS, CONTROLLERS } from '../../docs/assets/config-generator/schema.mjs';
import { buildConfigs } from '../../docs/assets/config-generator/model.mjs';
import { serialize } from '../../docs/assets/config-generator/serializers.mjs';

const dir = resolve('.cache/config-generator-fixtures');
mkdirSync(dir, { recursive: true });
// parse_config checks existence; these fixtures only exercise parsing, not file TLS.
writeFileSync(join(dir, 'certificate.fixture'), 'parser fixture');
writeFileSync(join(dir, 'key.fixture'), 'parser fixture');
let count = 0;
for (const [controller] of CONTROLLERS) {
  for (const tlsMode of ['certificate', 'acme', 'self']) {
    const s = initialState();
    Object.assign(s, {
      host: 'localhost', hostname: 'tuic.example.com', tlsMode, controller,
      insecure: tlsMode === 'self', email: 'admin@example.com',
      certificate: join(dir, 'certificate.fixture'), privateKey: join(dir, 'key.fixture'),
      dataDir: join(dir, 'acme-cache'),
    });
    s.users = [credentials(), credentials()]; s.activeUser = 1;
    if (controller !== 'bbr') {
      s.host = '[2001:db8::1]';
      s.users[1].password += ' " \\ \n 中文 😀 # [users]';
      Object.assign(s, { localAuth: true, localUsername: 'generator-test', localPassword: s.users[1].password, zeroRtt: true });
      s.forwards = [
        { protocol: 'tcp', listen: '127.0.0.1:8080', remote: 'example.com:80', timeout: '60' },
        { protocol: 'udp', listen: '[::1]:8053', remote: '[2001:db8::53]:53', timeout: '45' },
      ];
    }
    for (const [side, config] of Object.entries(buildConfigs(s))) {
      for (const format of FORMATS) {
        writeFileSync(join(dir, `${controller}-${tlsMode}-${side}.${format}`), serialize(config, format));
        count++;
      }
    }
  }
}
console.log(`Generated ${count} ephemeral parser fixtures`);
