const twgl = require('twgl.js');
const sketch = require('./../concentric');

const VERTEX_SHADER = require('./quad.vert.glsl');
const PASSTHROUGH_SHADER = require('./quad.frag.glsl');
const GAUSS_WEIGHTS = [0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216];

const BlurPass1 = {
  beforeRender: (state, uniforms) => {
    uniforms.horizontal = true;
    uniforms.weights = GAUSS_WEIGHTS;
  },
  fragmentShader: require('./blur.glsl'),
};

const BlurPass2 = {
  beforeRender: (state, uniforms) => {
    uniforms.horizontal = false;
    uniforms.weights = GAUSS_WEIGHTS;
  },
  fragmentShader: require('./blur.glsl'),
};

const MotionBlur = {
  fragmentShader: require('./motionblur.glsl'),
};

const renderConfig = [
  MotionBlur,
  BlurPass1,
  BlurPass2,
  {
    beforeRender: (state, uniforms) => {
      uniforms.contribution = 1.0;
    },
    fragmentShader: require('./blend.glsl'),
  },
];

function setup (state) {
  const width = 1920;
  const height = 1080;

  state.canvas.style['display'] = 'none';
  const canvas = document.createElement('canvas');
  canvas.id = 'app';
  canvas.width = width;
  canvas.height = height;
  document.body.appendChild(canvas);

  const gl = canvas.getContext('webgl');
  const childTexture = twgl.createTexture(gl, {src: state.canvas});
  const bufferInfo = twgl.createBufferInfoFromArrays(gl, {
    position: [
      -1, -1,  0,
       1, -1,  0,
      -1,  1,  0,
      -1,  1,  0,
       1, -1,  0,
       1,  1,  0
    ],
  });
  const frameBufferInfos = [
    twgl.createFramebufferInfo(gl),
    twgl.createFramebufferInfo(gl)
  ];

  // Final render pass, should present to the screen
  renderConfig.push({
    fragmentShader: PASSTHROUGH_SHADER,
  });

  // Build ping pong render passes
  const renderPasses = renderConfig.map((config, i) => {
    const {fragmentShader, vertexShader} = config;

    const programInfo = twgl.createProgramInfo(gl, [
      vertexShader || VERTEX_SHADER,
      fragmentShader
    ]);

    // Input from either the sketch texture or previous render pass
    const inputTexture = (i === 0) ?
      childTexture : (
      frameBufferInfos[(i + 0) % 2].attachments[0]);

    // Render to a frame buffer or the screen
    const outputFrameBuffer = (i === renderConfig.length - 1) ?
      undefined :
      frameBufferInfos[(i + 1) % 2];

    return {
      programInfo,
      inputTexture,
      outputFrameBuffer,
      beforeRender: config.beforeRender || function () {},
    };
  });

  const s = {
    gl,
    width,
    height,
    t: 0,

    bufferInfo,
    frameBufferInfos,
    renderPasses,

    childState: sketch.setup(state),
    childCanvas: state.canvas,
    childTexture: childTexture,
  };

  return s;
}

function draw (state, delta) {
  const {
    gl,
    frameBufferInfos,
    bufferInfo,
    width,
    height,
    renderPasses,
    childTexture,
    childState,
    childCanvas,
  } = state;

  // Render the sketch and update child texture from the hidden canvas
  sketch.draw(state.childState, delta);
  twgl.setTextureFromElement(gl, childTexture, childCanvas);

  state.t += delta;

  const uniforms = {
    time: state.t / 1000.0,
    resolution: [width, height],
    originalImage: childTexture,
  };

  gl.viewport(0, 0, width, height);
  renderPasses.forEach((renderPass, i) => {
    uniforms.inputTexture = renderPass.inputTexture;
    renderPass.beforeRender(state, uniforms);
    twgl.bindFramebufferInfo(gl, renderPass.outputFrameBuffer);
    gl.useProgram(renderPass.programInfo.program);
    twgl.setBuffersAndAttributes(gl, renderPass.programInfo, bufferInfo);
    twgl.setUniforms(renderPass.programInfo, uniforms);
    twgl.drawBufferInfo(gl, bufferInfo);
  });
}

function handlers () {
  if (!sketch.handlers) {
    return {};
  }

  const child = sketch.handlers();
  return {
    '/note_on': function (msg, state) {
      child['/note_on'](msg, state.childState);
    },
  };
}

module.exports = {
  setup,
  handlers,
  draw,
};
