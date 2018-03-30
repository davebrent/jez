function setup ({canvas, context}) {
  const width = 1920;
  const height = 1080;

  canvas.width = width;
  canvas.height = height;

  return {
    context,
    width,
    height,
    circles: [],
  };
}

function handlers () {
  return {
    '/note_on': function (msg, state) {
      const pitch = msg[1];

      if (pitch === 1) {
        const x = (state.width / 4) - (Math.random() * (state.width / 2));
        const y = (state.height / 4) - (Math.random() * (state.height / 2));
        const r = 500 + (Math.random() * 300);
        state.circles.push(new Circle(8000, x, y, r));
      } else if (pitch === 64) {
        const x = (state.width / 4) - (Math.random() * (state.width / 2));
        const y = (state.height / 4) - (Math.random() * (state.height / 2));
        const r = 500 + (Math.random() * 300);
        state.circles.push(new Circle(9000, x, y, r));
      } else {
        const x = (state.width / 2) - (Math.random() * state.width);
        const y = (state.height / 2) - (Math.random() * state.height);
        const r = 400 + (Math.random() * 100);
        state.circles.push(new Circle(4000, x, y, r));
      }
    }
  };
}

function draw (state, delta) {
  const {context, width, height, circles} = state;
  context.fillStyle = '#1e1e1e';
  context.fillRect(0, 0, width, height);

  context.save();
  context.translate(width / 2, height / 2);
  circles.forEach((c) => c.draw(context));
  circles.forEach((c) => c.update(delta));

  let len = circles.length;
  while (len--) {
    const c = circles[len];
    if (c.elapsed >= c.duration) {
      circles.splice(len, 1);
    }
  }

  context.restore();
}

module.exports = {
  setup,
  handlers,
  draw,
};

class Circle {
  constructor(duration, x, y, r) {
    const rand = () => ((Math.random() * 2) - 1);
    this.elapsed = 0;
    this.duration = duration;
    this.x = 0;
    this.y = 0;
    this.r = r;
    this.shrink = Math.random() > 0.5;
    this.rotation = toRadians(rand() * 360)
    this.startAngle = rand() * 4 * Math.PI;
    this.endAngle = rand() * 4 * Math.PI;
    this.lineWidth = (Math.random() * 60) + 2;
    if (this.startAngle > this.endAngle) {
      const t = this.startAngle;
      this.startAngle = this.endAngle;
      this.endAngle = t;
    }
    this.backTrack = Math.random() > 0.5;
  }

  update(delta) {
    this.elapsed += delta;
  }

  draw(context) {
    let {x, y, r} = this;
    let t = this.elapsed / this.duration;
    let alpha = t;
    if (alpha > 0.5) {
      alpha = 0.5 - (alpha - 0.5);
    }
    alpha *= 2;

    if (this.backTrack) {
      if (t > 0.5) {
        t = 0.5 - (t - 0.5);
      }
    }

    const realT = this.elapsed / this.duration;
    if (r > 500) {
      context.strokeStyle = `rgba(0, 255, 255, ${alpha})`;
    } else {
      context.strokeStyle = `rgba(255, 255, 255, ${alpha})`;
    }
    if (this.shrink) {
      context.lineWidth = this.lineWidth - (this.lineWidth * t);
    } else {
      context.lineWidth = this.lineWidth * t;
    }

    const numCircles = r / (60 + (0 * 60));
    let i = 0;
    while (r > 50) {
      const s = i / numCircles;
      i++;

      context.beginPath();
      if (this.shrink) {
        const radius = r - (r * t);
        context.ellipse(x, y, radius, radius, this.rotation * t, this.startAngle * t * s, this.endAngle * t * s);
      } else {
        const radius = r * t;
        context.ellipse(x, y, radius, radius, this.rotation * t, this.startAngle * t * s, this.endAngle * t * s);
      }
      context.stroke();
      r -= 60 + (0 * 60);
    }
  }
}

function toRadians (angle) {
  return angle * (Math.PI / 180);
}

function ellipse (context, x, y, r) {
  context.ellipse(x, y, r, r, 45 * Math.PI / 180, 0, 2 * Math.PI);
}
