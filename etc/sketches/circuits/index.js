function setup ({canvas, context}) {
  const width = 1920;
  const height = 1080;

  canvas.width = width;
  canvas.height = height;

  return {
    canvas,
    context,
    width,
    height,
    redraw: true,
  };
}


function draw (state) {
  const {context, width, height} = state;
  if (!state.redraw) {
    return;
  }

  context.fillStyle = '#212121';
  context.fillRect(0, 0, width, height);

  context.strokeStyle = 'rgba(255, 255, 21, 0.6)';
  for (let i = 0; i < 8; ++i) {
    context.save();
      context.translate(width / 2, height / 2);
      segment(context, toRadians(90), 0);
    context.restore();
  }

  context.strokeStyle = 'rgba(255, 255, 255, 0.6)';
  for (let i = 0; i < 8; ++i) {
    context.save();
      context.translate(width / 2, height / 2);
      segment(context, toRadians(60), 0);
    context.restore();
  }

  state.redraw = false;
}


function handlers () {
  return {
    '/note_on': function (msg, state) {
      state.redraw = true;
    },
  };
}

module.exports = {
  setup,
  handlers,
  draw,
};

function toRadians (angle) {
  return angle * (Math.PI / 180);
}

function line (context, x1, y1, x2, y2) {
  context.beginPath();
  context.moveTo(x1, y1);
  context.lineTo(x2, y2);
  context.stroke();
}

function ellipse (context, x, y, r) {
  context.ellipse(x, y, r, r, 45 * Math.PI / 180, 0, 2 * Math.PI);
}

function segment (context, theta, iter) {
  // As well as changing stroke also add possibility of concentric lines
  let len = 2.5;
  var dir = 0;
  if (Math.random() > 0.66) {
    dir = (Math.random() > 0.5) ? -1 : 1;
  }
  if (iter > 50) {
    return;
  }

  let stroke = (Math.random() > 0.95) ? 6 : 1;
  context.lineWidth = stroke;

  let concentric = Math.random() > 0.95;
  let ellipseP = Math.random() > 0.95;

  let i = Math.round(Math.random() * 2) - 1;
  if (iter > 30) {
    let x = iter * len * dir;
    if (!ellipseP) {
      if (concentric) {
        line(context, 0, -10, x, -10);
        line(context, 0, 10, x, 10);
      }
      line(context, 0, 0, x, 0);
    } else {
      let r = Math.random() * 30;
      context.beginPath();
      if (concentric) {
        let s = r * 0.25;
        ellipse(context, 0, x, r - s);
        ellipse(context, 0, x, r - s - s);
      }
      ellipse(context, 0, x, r);
      context.stroke();
    }
  }
  context.translate(iter * len * dir, 0);

  context.save();
  context.rotate(-(theta * i));
  segment(context, theta, iter + 1);
  context.restore();
}
