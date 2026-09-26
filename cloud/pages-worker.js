// Dedicated test Pages project only. Production's feature-request Worker is separate.
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    // Reject pages.dev, immutable deployments and branch aliases, even for static files.
    if (url.origin !== env.APP_ORIGIN) return new Response('Use the protected test hostname.', { status: 403 });
    if (url.pathname.startsWith('/api/')) return env.CHARACTERS.fetch(request);
    return env.ASSETS.fetch(request);
  },
};
