function setup ({canvas, context}) {
  const width = 1920;
  const height = 1080;

  canvas.width = width;
  canvas.height = height;

  return {
    t: 0,
    canvas,
    context,
    width,
    height,
    gridSize: 8,
    tileSize: 72,
    nums: [],
  }
}

function draw (state) {
  const {context, width, height} = state;
  context.fillStyle = '#212121';
  context.fillRect(0, 0, width, height);

  isometricCenter(state, (state) => isometric(state, blocks));
  isometricCenter(state, (state) => isometric(state, grid));
  orthographicCenter(state, blocks);
  orthographicCenter(state, grid);
  update(state);
}

module.exports = {
  setup,
  draw,
};

function binary (dec) {
  let bits = (dec).toString(2).split('');
  while (bits.length !== 8) {
    bits.unshift('0');
  }
  return bits.map((bit) => parseInt(bit, 2));
}

function isometric (state, func) {
  const {context, width, height} = state;

  context.save();
    context.scale(1, 0.5);
    context.rotate((45 * Math.PI) / 180);
    func(state)
  context.restore();
}

function isometricCenter (state, func) {
  const {context, width, height, gridSize, tileSize} = state;
  const [x1, y1] = project(0, 0, tileSize);
  const [x2, y2] = project(gridSize, gridSize, tileSize);
  const offset = (height - (y2 - y1)) / 2;

  context.save();
    context.translate((width / 2) + (width / 4), offset);
    func(state)
  context.restore();
}

function orthographicCenter (state, func) {
  const {context, width, height, gridSize, tileSize} = state;
  const x = (width - (gridSize * (tileSize * 0.66))) / 2;
  const y = (height - (gridSize * (tileSize * 0.66))) / 2;

  context.save();
    context.translate(x - (width / 4), y);
    context.scale(0.66, 0.66);
    func(state)
  context.restore();
}

function rotate (x, y, angle) {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  return [
    (x * c) - (y * s),
    (x * s) + (y * c)
  ];
}

function project (x, y, tileSize) {
  x *= tileSize;
  y *= tileSize;
  let [rx, ry] = rotate(x, y, (45 * Math.PI) / 180);
  rx *= 1.0;
  ry *= 0.5;
  return [rx, ry];
}

function line (context, x1, y1, x2, y2) {
  context.beginPath();
  context.moveTo(x1, y1);
  context.lineTo(x2, y2);
  context.stroke();
}

function update (state) {
  if (state.t % 16 === 0) {
    const num = Math.round((Math.random() * 255) + 0);
    for (let i = 0; i < 2; ++i) {
      state.nums.push(num);
    }
  }

  while (state.nums.length > state.gridSize) {
    state.nums.shift();
  }

  state.t += 1;
}

function blocks (state) {
  const {context, t, nums, gridSize, tileSize} = state;
  context.fillStyle = 'white';
  nums.forEach((num, i) => {
    binary(num).forEach((bit, k) => {
      if (bit === 1) {
        context.fillRect(k * tileSize, i * tileSize, tileSize, tileSize);
      }
    });
  });
}

function debugLines ({context, gridSize, tileSize}) {
  context.lineWidth = 3;

  context.strokeStyle = 'red';
  const [x1, y1] = project(0, 0, tileSize);
  const [x2, y2] = project(gridSize, gridSize, tileSize);
  line(context, x1, y1, x2, y2);

  context.strokeStyle = 'pink';
  const [x11, y11] = project(0, 4, tileSize);
  const [x21, y21] = project(gridSize, 4, tileSize);
  line(context, x11, y11, x21, y21);
}

function grid ({context, gridSize, tileSize}) {
  context.strokeStyle = 'rgba(200, 200, 200, 0.2)';
  context.font = '20px sans-serif';
  context.fillStyle = 'white';
  context.textAlign = 'center';

  for (let y = 0; y < gridSize; y++) {
    for (let x = 0; x < gridSize; x++) {
      let dx = x * tileSize;
      let dy = y * tileSize;
      let text = `${x},${y}`;
      context.strokeRect(dx, dy, tileSize, tileSize);
      context.fillText(text, dx + (tileSize / 2), dy + (tileSize / 2) + 10);
    }
  }
}
