const number = location.pathname.match(/^\/([1-9][0-9]{0,8})\/?$/)?.[1];
const title = document.getElementById('ticket-title');
const status = document.getElementById('ticket-status');
const frame = document.getElementById('ticket-preview');
const form = document.getElementById('revision-form');
const submit = document.getElementById('revision-submit');
const feedback = document.getElementById('revision-status');
let current, challenge, challengeReady = 0, challengeExpires = 0, sending = false, timer;
const labels = { queued: 'Waiting to start', coding: 'Work in progress', building: 'Building and testing', ready: 'Preview ready', failed: 'Needs attention', 'needs-info': 'More detail needed', closed: 'Closed' };
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
  } catch (error) { status.textContent = error.message || 'Couldn’t check progress. Trying again shortly…'; }
  if (keepPolling) timer = setTimeout(update, 30000);
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
