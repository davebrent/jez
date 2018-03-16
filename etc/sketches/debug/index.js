const smoothie = require('smoothie');

function setup ({canvas}) {
  var timeseries = new smoothie.TimeSeries();
  var smooth = new smoothie.SmoothieChart({
    minValue: 0,
    maxValue: 127,
    interpolation: 'linear',
    millisPerPixel: 5,
  });

  smooth.addTimeSeries(timeseries, {
    lineWidth: 2,
    strokeStyle: 'red',
  });

  smooth.streamTo(canvas);

  return {
    smooth,
    timeseries,
  };
}

function handlers () {
  return {
    '/ctrl': function ([chan, ctrl, val], {timeseries}) {
      timeseries.append(Date.now(), val);
    }
  };
}

function draw () {
}

module.exports = {
  setup,
  handlers,
  draw
};
