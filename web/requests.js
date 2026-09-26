const panel = document.getElementById('feature-requests');
const form = document.getElementById('request-form');
const button = document.getElementById('request-submit');
const status = document.getElementById('request-status');
let challenge;
let initialized = false;
let submitting = false;
let readyAt = 0;

async function initialize() {
  if (location.hostname !== 'feature.sim-gui.com') {
    form.hidden = true;
    document.getElementById('request-location').hidden = false;
    return;
  }
  try {
    const response = await fetch('/api/request-challenge', { cache: 'no-store' });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error);
    challenge = data.challenge;
    readyAt = Date.now() + 3200;
    status.textContent = 'Ready when you are.';
    setTimeout(() => { if (!submitting) button.disabled = false; }, 3200);
  } catch (error) {
    status.textContent = error.message || 'Couldn’t load the form. Try reloading the page.';
    status.dataset.error = 'true';
  }
}

export function showFeatureRequests(visible, top = 0) {
  panel.hidden = !visible;
  panel.style.top = `${Math.max(0, top)}px`;
  if (visible && !initialized) { initialized = true; void initialize(); }
}

window.addEventListener('sim-feature-tab', ({ detail }) =>
  showFeatureRequests(Boolean(detail.visible), detail.top));

form.addEventListener('submit', async event => {
  event.preventDefault();
  if (!challenge || submitting || Date.now() < readyAt) return;
  submitting = true;
  button.disabled = true;
  status.dataset.error = 'false';
  status.textContent = 'Sending your request…';
  try {
    const response = await fetch('/api/feature-requests', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: form.elements.title.value, description: form.elements.description.value,
        website: form.elements.website.value, challenge }),
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error);
    if (!Number.isSafeInteger(data.number) || data.number < 1) throw new Error('Couldn’t confirm your request. Please try again.');
    status.textContent = `Request #${data.number} ${data.duplicate ? 'already exists' : 'sent'}. `;
    const link = document.createElement('a');
    link.href = `/${data.number}`;
    link.textContent = 'View progress and preview';
    status.append(link);
    button.textContent = 'Request submitted';
    form.elements.title.readOnly = true;
    form.elements.description.readOnly = true;
  } catch (error) {
    status.textContent = error.message || 'Couldn’t send your request. Please try again.';
    status.dataset.error = 'true';
    submitting = false;
    button.disabled = false;
  }
});
