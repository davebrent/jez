// A script to encode sketches, sequenced by Jez, to HD video
//
// Run Jez from a child process in simulation mode, writing the program to
// stdin, reading commands from stdout. Load the sketch using `node-canvas` and
// run it, calling the sketches message handlers where appropriate. After each
// frame write the node canvas data (encoded as a png) to ffmpeg in another
// child processs. Then... $$$?

// https://medium.com/@brianshaler/on-the-fly-video-rendering-with-node-js-and-ffmpeg-165590314f2
// https://scwu.io/blog/2015/09/11/Rendering-Canvas-to-mp4-using-nodejs/

const {spawn} = require('child_process');
const {readFileSync} = require('fs');
const {join} = require('path');
const {createCanvas} = require('canvas');


const FFMPEG_BIN = 'ffmpeg';
const JEZ_BIN = '/home/admin/Shared/jez/target/release/jez';


// Run jez in simulation mode to get all the commads 'faster than realtime'
function runProgram (sketchName, duration) {
  return new Promise((resolve, reject) => {
    const jez = spawn(JEZ_BIN, [
      '--simulate',
      `--time=${duration}`
    ]);

    let stdout = '';
    let stderr = '';

    jez.stdout.on('data', (data) => stdout += data.toString('utf-8'));
    jez.stderr.on('data', (err) => stderr += err.toString('utf-8'));
    jez.stdout.on('close', (err) => {
      if (!err) {
        resolve(JSON.parse(stdout).commands);
      } else {
        reject({stderr, stdout});
      }
    });

    jez.stdin.write(readFileSync(join(__dirname, SKETCH_NAME, 'index.jez')));
    jez.stdin.end();
  });
}

// Map jez commands to something that looks like an osc message
function encodeCommands (output, cmd) {
  const name = typeof cmd === 'string' ? cmd : Object.keys(cmd)[0];
  if (name === 'Event') {
    if (cmd.Event.value.Trigger) {
      const event = cmd.Event;
      const [channel, velocity] = event.dest.Midi;
      output.push([
        event.onset,
        { addr: '/note_on', args: [ channel, event.value.Trigger, velocity ] }
      ]);
      output.push([
        event.onset + event.dur,
        { addr: '/note_off', args: [ channel, event.value.Trigger ] }
      ]);
    } else {
      throw new Error('Unsupported');
    }
  }
  return output;
}

function runSketch (sketchName, delta, duration, msgs) {
  const canvas = createCanvas(1920, 1080);
  const context = canvas.getContext('2d');

  const sketchPath = join(__dirname, SKETCH_NAME);
  const sketch = require(sketchPath);
  const state = sketch.setup({canvas, context});

  const ffmpeg = spawn(FFMPEG_BIN, [
    '-hide_banner',
    '-f', 'image2pipe',
    // Input options
    '-vcodec', 'png',
    '-r', '60',
    '-i', '-',
    // Output options, see https://vimeo.com/help/compression
    '-vcodec', 'h264',
    '-profile', 'high',
    '-pix_fmt', 'yuv420p',
    // Override output file if it exists
    '-y', 'output.mp4',
  ]);

  ffmpeg.stdout.on('data', (err) => console.log(err.toString('utf-8')));
  ffmpeg.stderr.on('data', (err) => console.log(err.toString('utf-8')));

  let elapsed = 0;
  let handlers = sketch.handlers();

  (function next() {
    console.log(elapsed);
    if (elapsed > duration) {
      ffmpeg.stdin.end();
      return;
    }

    while (msgs.length) {
      const [t, msg] = msgs[0];
      if (t <= elapsed) {
        msgs.shift();
        if (handlers[msg.addr]) {
          handlers[msg.addr](msg.args, state);
        }
      } else {
        break;
      }
    }

    sketch.draw(state, delta);
    elapsed += delta;

    canvas.pngStream()
      .on('end', next)
      .pipe(ffmpeg.stdin, {end: false});
  }());
}

const SKETCH_NAME = 'scanlines';
const DURATION = 90000;

runProgram(SKETCH_NAME, DURATION)
  .then((cmds) => cmds.reduce(encodeCommands, []))
  .then((msgs) => runSketch(SKETCH_NAME, 1000 / 60, DURATION, msgs));
