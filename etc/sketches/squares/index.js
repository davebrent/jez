function setup ({context, canvas}) {
  const width = 1920;
  const height = 1080;

  canvas.width = width;
  canvas.height = height;

  return {
    width,
    height,
    canvas,
    context,
    shapes: [],
  };
}

function draw (state, delta) {
  const {context, shapes, width, height} = state;

  if (!shapes.length) {
    const dur = 4000;
    const x = (state.width / 4) - (Math.random() * (state.width / 2));
    const y = (state.height / 4) - (Math.random() * (state.height / 2));
    const w = 300 + (Math.random() * 600);
    const h = 300 + (Math.random() * 600);
    shapes.push(new Square(dur, x, y, w, h));
  }

  context.fillStyle = 'rgba(0, 0, 0, 1)';
  context.fillRect(0, 0, width, height);

  context.save();
  context.translate(width / 2, height / 2);
  shapes.forEach((c) => c.draw(context));
  shapes.forEach((c) => c.update(delta));

  let len = shapes.length;
  while (len--) {
    const s = shapes[len];
    if (s.elapsed >= s.duration) {
      shapes.splice(len, 1);
    }
  }

  context.restore();
}

module.exports = {
  setup,
  draw,
};

function radians (angle) {
  return angle * (Math.PI / 180);
}

class Square {
  constructor(duration, x, y, w, h) {
    this.x = 0;
    this.y = 0;
    this.w = w;
    this.h = h;

    this.rotation = 0;
    this.elapsed = 0;
    this.duration = duration;
  }

  update(delta) {
    this.elapsed += delta;
    this.rotation += radians(this.duration / 360);
  }

  draw(context) {
    let {x, y, w, h} = this;
    let t = this.elapsed / this.duration;
    if (t > 0.5) {
      t = 0.5 - (t - 0.5);
    }
    t = Math.pow(t * 2, 2);

    context.save();
    context.rotate(this.rotation);
    context.translate(-((w * 1) / 2), -((h * 1) / 2));
    context.fillStyle = `rgba(255, 255, 255, ${t})`;
    context.fillRect(x, y, w * 1, h * 1);
    context.restore();
  }
}
