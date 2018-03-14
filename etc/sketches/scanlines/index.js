const PRIMARY = 'yellow';

function setup ({canvas, context}) {
  var state = {
    width: 1920,
    height: 1080,
    canvas: canvas,
    context: context,
    background: '#212121',
  };

  canvas.width = state.width;
  canvas.height = state.height;

  var x = 0;
  var y = 0;
  var w = state.width;
  var h = state.height;
  var s = 8;

  state.lines = [
    horizontal(s * -1, 1, 'white', x, y, w, h),
    vertical(s * -1, 1, 'white', x, y, w, h),
    horizontal(s, 1, 'white', x, y, w, h),
    vertical(s, 1, 'white', x, y, w, h),

    horizontal(s * -1.3, 1, 'white', x, y, w, h),
    vertical(s * -1.3, 1, 'white', x, y, w, h),
    horizontal(s, 1.3, 'white', x, y, w, h),
    vertical(s, 1.3, 'white', x, y, w, h),
  ];

  return state;
}

function handlers () {
  return {
    '/note_on': function ([channel, pitch, velocity], {lines}) {
      for (let line of lines) {
        line.acceleration += line.speed;
      }
      let i = Math.floor(Math.random() * lines.length);
      lines[i].stroke = (1 + lines[i].acceleration) * 6;
      if (pitch > 70) {
        lines[i].fillStyle = PRIMARY;
        lines[i].stroke *= lines[i].acceleration * 2;
      } else {
        lines[i].fillStyle = 'white';
      }
    },
    '/note_off': function ([channel, pitch], {lines}) {
      for (let line of lines) {
        line.acceleration -= line.speed;
        line.stroke = 1;
        line.reset();
        line.fillStyle = '#616161';
        line.fillStyle = 'rgba(0, 0, 0, 0)';
        line.fillStyle = 'yellow';
      }
    }
  };
}

function draw (state) {
  let {background, context, width, height, lines} = state;
  for (let line of lines) {
    line.update(state);
  }

  context.fillStyle = background;
  context.fillRect(0, 0, width, height);

  for (let line of lines) {
    line.render(state);
  }
}

module.exports = {
  setup,
  handlers,
  draw
};

function horizontal (speed, acceleration, fillStyle) {
  return {
    fillStyle: fillStyle,
    acceleration: acceleration,
    stroke: 1,
    position: 0,
    speed: speed,

    reset: function () {
      this.acceleration = acceleration;
    },

    update: function (state) {
      let {width} = state;
      this.position = (this.position + this.acceleration) % width;
      if (this.position < 0) {
        this.position = width;
      }
    },

    render: function (state) {
      var {context, height} = state;
      state.context.fillStyle = this.fillStyle;
      context.fillRect(this.position, 0, this.stroke, height);
    }
  };
}

function vertical (speed, acceleration, fillStyle) {
  return {
    fillStyle: fillStyle,
    acceleration: acceleration,
    stroke: 1,
    position: 0,
    speed: speed,

    reset: function () {
      this.acceleration = acceleration;
    },

    update: function (state) {
      let {height} = state;
      this.position = (this.position + this.acceleration) % height;
      if (this.position < 0) {
        this.position = height;
      }
    },

    render: function (state) {
      var {context, width} = state;
      state.context.fillStyle = this.fillStyle;
      context.fillRect(0, this.position, width, this.stroke);
    }
  };
}
