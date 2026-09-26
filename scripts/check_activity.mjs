// Manual deployment smoke check. A mismatched run/revision MUST authenticate but cannot write.
const origin = 'https://feature.sim-gui.com';
if (!/^[1-9][0-9]{0,8}$/.test(process.env.ISSUE_NUMBER || '')) throw new Error('Provide an existing signed ticket');
const url = new URL(process.env.ACTIONS_ID_TOKEN_REQUEST_URL);
url.searchParams.set('audience', `${origin}/activity`);
const identity = await fetch(url, { headers: { Authorization: `Bearer ${process.env.ACTIONS_ID_TOKEN_REQUEST_TOKEN}` }, signal: AbortSignal.timeout(10000), redirect: 'error' });
if (!identity.ok) throw new Error(`GitHub identity failed (${identity.status})`);
const { value } = await identity.json();
const response = await fetch(`${origin}/api/tickets/${process.env.ISSUE_NUMBER}/activity`, {
  method: 'POST', headers: { Authorization: `Bearer ${value}`, 'Content-Type': 'application/json' },
  body: JSON.stringify({ revision: -1, events: [{ id: 1, kind: 'message', text: 'Authentication smoke check.', at: new Date().toISOString() }] }),
  signal: AbortSignal.timeout(20000), redirect: 'error',
});
await response.body?.cancel();
if (response.status !== 409) throw new Error(`Expected authenticated run mismatch (409), received ${response.status}`);
console.log('Live GitHub identity accepted; wrong-run update rejected. No ticket modified.');
