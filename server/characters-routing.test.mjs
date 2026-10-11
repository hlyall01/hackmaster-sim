import test from 'node:test';
import assert from 'node:assert/strict';
import worker from './requests.mjs';

test('production routes character reads and writes to the character service unchanged', async () => {
  for (const path of ['/api/session', '/api/login', '/api/users', '/api/roster', '/api/characters/abc/owner', '/guest/characters', '/guest/characters/abc']) {
    for (const method of ['GET', 'PUT']) {
      const request = new Request(`https://sim-gui.com${path}`, {method});
      const response = await worker.fetch(request, {CHARACTERS: {fetch: async forwarded => {
        assert.equal(forwarded, request);
        return new Response('character service');
      }}});
      assert.equal(await response.text(), 'character service');
    }
  }
});

test('character routes fail closed without the service and on other hostnames', async () => {
  assert.equal((await worker.fetch(new Request('https://sim-gui.com/api/session'), {})).status, 503);
  for (const host of ['feature.sim-gui.com', 'hackmaster-sim.pages.dev', 'preview.hackmaster-sim-previews.pages.dev']) {
    const response = await worker.fetch(new Request(`https://${host}/guest/characters`), {
      CHARACTERS: {fetch: () => { throw new Error('Must not call character service'); }},
    });
    assert.equal(response.status, 403);
  }
});

test('feature tickets and static simulator assets retain their existing routes', async () => {
  const env = {ASSETS: {fetch: async request => new Response(new URL(request.url).pathname)}};
  const oldTicket = await worker.fetch(new Request('https://sim-gui.com/12'), env);
  assert.equal(oldTicket.status, 308);
  assert.equal(oldTicket.headers.get('Location'), 'https://feature.sim-gui.com/12');
  assert.equal(await (await worker.fetch(new Request('https://feature.sim-gui.com/12'), env)).text(), '/ticket');
  assert.equal(await (await worker.fetch(new Request('https://sim-gui.com/'), env)).text(), '/');
});
