// Ephemeral test keys; no live credentials or network access.
import { generateKeyPairSync } from 'node:crypto';
export const testKeys = generateKeyPairSync('rsa', { modulusLength: 2048 });
export const appEnv = {
  GITHUB_APP_ID: '123', GITHUB_APP_INSTALLATION_ID: '456',
  GITHUB_APP_PRIVATE_KEY: testKeys.privateKey.export({ type: 'pkcs8', format: 'pem' }),
};
export const tokenResponse = () => Response.json({ token: 'test-installation-token',
  expires_at: new Date(Date.now() + 3600000).toISOString() });
