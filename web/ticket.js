const number = location.pathname.match(/^\/([1-9][0-9]{0,8})\/?$/)?.[1];
const title = document.getElementById('ticket-title');
const status = document.getElementById('ticket-status');
const frame = document.getElementById('ticket-preview');
const labels = { queued: 'Queued', coding: 'Agent is working', building: 'Building and testing', ready: 'Preview ready', failed: 'Needs attention', 'needs-info': 'Needs clarification', closed: 'Completed or closed' };
function link(id, url) {
  const element = document.getElementById(id);
  element.hidden = !url;
  if (url) element.href = url;
}
async function update() {
  if (!number) { title.textContent = 'Ticket not found'; status.textContent = 'Open a ticket from the feature request form.'; return; }
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
    keepPolling = !['closed', 'failed', 'needs-info'].includes(ticket.status);
  } catch (error) { status.textContent = error.message || 'Unable to check status. Retrying shortly…'; }
  if (keepPolling) setTimeout(() => { if (!document.hidden) void update(); else setTimeout(update, 30000); }, 30000);
}
void update();
