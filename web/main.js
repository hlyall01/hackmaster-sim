import { showFeatureRequests } from './requests.js?v=feature-domain-1';

if (location.hostname === 'feature.sim-gui.com') {
  document.title = 'Feature requests · HackMaster Simulator';
  document.getElementById('sim_canvas').hidden = true;
  document.getElementById('loading').hidden = true;
  document.getElementById('portal-navigation').hidden = false;
  showFeatureRequests(true);
} else if (location.hostname === 'sim-gui.com' && new URLSearchParams(location.search).get('tab') === 'requests') {
  location.replace('https://feature.sim-gui.com/');
} else try {
  const { default: init, start_web } = await import('./pkg/sim_gui.js');
  await init();
  await start_web(new URLSearchParams(location.search).get('tab') === 'requests');
  document.getElementById('loading').hidden = true;
} catch (error) {
  console.error('Simulator startup failed', error);
  document.getElementById('status').textContent =
    'Unable to start the simulator. Please use a browser with WebAssembly and WebGL enabled, then reload. ' + String(error);
}
