const panel = document.getElementById('feature-requests');
const form = document.getElementById('request-form');
const button = document.getElementById('request-submit');
const status = document.getElementById('request-status');
let challenge;
let initialized = false;
let submitting = false;
let readyAt = 0;

async function initialize() {
  if (location.hostname !== 'sim-gui.com') {
    status.textContent = 'Submit new requests on sim-gui.com.';
    const link = document.createElement('a');
    link.href = 'https://sim-gui.com/?tab=requests';
    link.textContent = ' Open feature requests';
    status.append(link);
    return;
  }
  try {
    const response = await fetch('/api/request-challenge', { cache: 'no-store' });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error);
    challenge = data.challenge;
    readyAt = Date.now() + 3200;
    status.textContent = 'Describe one improvement per request.';
    setTimeout(() => { if (!submitting) button.disabled = false; }, 3200);
  } catch (error) {
    status.textContent = error.message || 'Unable to load the form. Please reload the page.';
    status.dataset.error = 'true';
  }
}

window.addEventListener('sim-feature-tab', ({ detail }) => {
  const visible = Boolean(detail.visible);
  panel.hidden = !visible;
  panel.style.top = `${Math.max(0, detail.top)}px`;
  if (visible && !initialized) { initialized = true; void initialize(); }
});

form.addEventListener('submit', async event => {
  event.preventDefault();
  if (!challenge || submitting || Date.now() < readyAt) return;
  submitting = true;
  button.disabled = true;
  status.dataset.error = 'false';
  status.textContent = 'Creating your ticket…';
  try {
    const response = await fetch('/api/feature-requests', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: form.elements.title.value, description: form.elements.description.value,
        website: form.elements.website.value, challenge }),
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error);
    if (!Number.isSafeInteger(data.number) || data.number < 1) throw new Error('Unexpected ticket response.');
    status.textContent = `Ticket #${data.number} ${data.duplicate ? 'already exists' : 'created'}. `;
    const link = document.createElement('a');
    link.href = `/${data.number}`;
    link.textContent = 'Follow the agent and open your preview';
    status.append(link);
    button.textContent = 'Request submitted';
    form.elements.title.readOnly = true;
    form.elements.description.readOnly = true;
  } catch (error) {
    status.textContent = error.message || 'Could not create the ticket. Please try again.';
    status.dataset.error = 'true';
    submitting = false;
    button.disabled = false;
  }
});
