try {
  const { default: init, start_web } = await import('./pkg/sim_gui.js');
  await init();
  await start_web();
  document.getElementById('loading').hidden = true;
} catch (error) {
  console.error('Simulator startup failed', error);
  document.getElementById('status').textContent =
    'Unable to start the simulator. Please use a browser with WebAssembly and WebGL enabled, then reload. ' + String(error);
}
