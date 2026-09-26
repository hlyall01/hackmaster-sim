const number = location.pathname.match(/^\/([1-9][0-9]{0,8})\/?$/)?.[1];
const title = document.getElementById('ticket-title');
const status = document.getElementById('ticket-status');
const frame = document.getElementById('ticket-preview');
const form = document.getElementById('revision-form');
const submit = document.getElementById('revision-submit');
const feedback = document.getElementById('revision-status');
let current, challenge, challengeReady = 0, challengeExpires = 0, sending = false, timer;
const labels = { queued: 'Waiting to start', screening: 'Checking request', rejected: 'Request rejected', coding: 'Work in progress', building: 'Building and testing', ready: 'Preview ready', failed: 'Needs attention', 'needs-info': 'More detail needed', closed: 'Closed' };
let activityKey = '';
function renderActivity(ticket) {
  const active = ['queued', 'screening', 'coding', 'building'].includes(ticket.status);
  const events = ticket.activity?.events || [];
  const stale = active && ticket.activity && Date.now() - Date.parse(ticket.activity.updated) > 90000;
  document.getElementById('activity-state').textContent = stale ? 'Waiting for an update' : labels[ticket.status] || '';
  document.getElementById('activity-note').textContent = events.length
    ? `${active ? 'Checks for updates every 15 seconds. ' : ''}Showing the latest activity from this run.${stale ? ' The agent may be busy with a longer task.' : ''}`
    : active ? 'Waiting for agent messages. This panel updates automatically.' : 'No detailed activity was recorded for this run.';
  link('activity-run', ticket.run);
  const nextKey = JSON.stringify([ticket.revision, ticket.run, events]);
  if (nextKey === activityKey) return;
  activityKey = nextKey;
  const list = document.getElementById('activity-list');
  const follow = list.scrollHeight - list.scrollTop - list.clientHeight < 50;
  list.replaceChildren(...events.map(event => {
    const item = document.createElement('li');
    item.className = `activity-${event.kind}`;
    const time = document.createElement('time');
    time.dateTime = event.at;
    time.textContent = new Date(event.at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const text = document.createElement('p');
    text.textContent = event.text;
    item.append(time, text);
    return item;
  }));
  if (follow) list.scrollTop = list.scrollHeight;
}
function link(id, url) {
  const element = document.getElementById(id);
  element.hidden = !url;
  if (url) element.href = url;
}
async function update() {
  clearTimeout(timer);
  if (!number) { title.textContent = 'Request not found'; status.textContent = 'Use the link you received after sending your request.'; return; }
  let keepPolling = true;
  try {
    const response = await fetch(`/api/tickets/${number}`, { cache: 'no-store' });
    const ticket = await response.json();
    if (!response.ok) { if (response.status === 404) keepPolling = false; throw new Error(ticket.error); }
    title.textContent = `#${number} · ${ticket.title.replace(/^\[Request\] /, '')}`;
    document.title = `#${number} · HackMaster Simulator`;
    status.textContent = `${labels[ticket.status] || 'In progress'}. ${ticket.message}`;
    link('ticket-issue', ticket.issue);
    link('ticket-pr', ticket.pr);
    link('ticket-open', ticket.preview);
    frame.hidden = !ticket.preview;
    if (ticket.preview && frame.getAttribute('src') !== ticket.preview) frame.src = ticket.preview;
    current = ticket;
    renderActivity(ticket);
    document.getElementById('revision-section').hidden = false;
    document.getElementById('revision-availability').textContent = ticket.status === 'closed'
      ? 'This request is closed. Start a new feature request for further changes.'
      : ticket.canRevise ? 'One request every 10 minutes, up to five per day per connection.'
      : 'You can send more changes when the current work finishes. The last available preview stays here while we work.';
    form.hidden = ticket.status === 'closed';
    const history = ticket.history || [];
    document.getElementById('revision-history').hidden = !history.length;
    document.getElementById('revision-list').replaceChildren(...history.map(item => {
      const li = document.createElement('li');
      li.textContent = item.description;
      return li;
    }));
    if (ticket.canRevise && (!challenge || Date.now() >= challengeExpires)) await prepareForm();
    enableSubmit();
    keepPolling = ticket.status !== 'closed';
  } catch (error) {
    status.textContent = error.message || 'Couldn’t check progress. Trying again shortly…';
    document.getElementById('activity-state').textContent = 'Reconnecting…';
  }
  if (keepPolling) timer = setTimeout(update, ['queued', 'screening', 'coding', 'building'].includes(current?.status) ? 15000 : 30000);
}
function enableSubmit() {
  submit.disabled = sending || !current?.canRevise || !challenge || Date.now() < challengeReady || Date.now() >= challengeExpires;
}
async function prepareForm() {
  challenge = null;
  enableSubmit();
  const response = await fetch(`/api/request-challenge?scope=revision:${number}`, { cache: 'no-store' });
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || 'Could not prepare the form.');
  challenge = data.challenge;
  challengeReady = Date.now() + 3200;
  challengeExpires = Date.now() + 3500_000;
  setTimeout(enableSubmit, 3250);
}
form.addEventListener('submit', async event => {
  event.preventDefault();
  if (submit.disabled) return;
  sending = true;
  enableSubmit();
  feedback.dataset.error = 'false';
  feedback.textContent = 'Sending your changes…';
  try {
    const response = await fetch(`/api/tickets/${number}/revisions`, { method: 'POST',
      headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({
        description: form.elements.description.value, website: form.elements.website.value,
        challenge, revision: current.revision,
      }) });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || 'Could not send your changes.');
    form.reset();
    challenge = null;
    current.canRevise = false;
    feedback.textContent = 'Changes requested. This page will update automatically when the new preview is ready.';
    await update();
  } catch (error) {
    feedback.dataset.error = 'true';
    feedback.textContent = error.message;
    // Retain the draft and token: retrying a lost response returns the same revision.
    await update();
  } finally { sending = false; enableSubmit(); }
});
void update();
