const OSC = require('osc-js');
const sketch = require('./scanlines');


const canvas = document.getElementById('app');
const context = canvas.getContext('2d');
const state = sketch.setup({canvas, context});
let previous = Date.now();

if (sketch.draw) {
  (function loop () {
    const now = Date.now();
    const delta = now - previous;
    previous = now;
    sketch.draw(state, delta);
    requestAnimationFrame(loop);
  }());
}

if (sketch.handlers) {
  const osc = new OSC();
  const handlers = sketch.handlers();
  for (let [addr, fn] of Object.entries(handlers)) {
    osc.on(addr, (message) => fn(message.args, state));
  }
  osc.open({port: 2794, host: '127.0.0.1'});
}
