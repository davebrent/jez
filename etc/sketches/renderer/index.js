const twgl = require('twgl.js');
const {demo} = require('./ecs');

const entityVertexShader = `
attribute vec3 position;
attribute vec3 normal;
attribute vec2 texcoord;

uniform mat4 modelViewProjection;
varying vec3 vNormal;
varying vec2 vUv;

void main() {
  gl_Position = modelViewProjection * vec4(position, 1.0);
  vNormal = normal;
  vUv = texcoord;
}
`;

const entityFragmentShader = `
precision mediump float;

uniform sampler2D grungeTexture;

varying vec3 vNormal;
varying vec2 vUv;

void main() {
  gl_FragColor = vec4(vNormal, 1.0) * texture2D(grungeTexture, vUv);
}
`;

const VERTEX_SHADER = require('./quad.vert.glsl');
const PASSTHROUGH_SHADER = require('./quad.frag.glsl');
const GAUSS_WEIGHTS = [0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216];

function radians (angle) {
  return angle * (Math.PI / 180);
}

class Object3D {
  constructor () {
    this.modelMatrix = twgl.m4.identity();
  }
}

class Camera extends Object3D {
  constructor (fov, aspect, near, far) {
    super();
    this.projectionMatrix = twgl.m4.perspective(radians(fov), aspect, near, far);
  }

  lookAt (eye, target, up) {
    twgl.m4.lookAt(eye, target, up, this.modelMatrix);
    return this;
  }
}

class Cube extends Object3D {
  constructor(size, position) {
    super();
    this.arrays = twgl.primitives.createPlaneVertices(size);
    twgl.m4.translate(this.modelMatrix, position, this.modelMatrix);
    twgl.m4.rotateX(this.modelMatrix, radians(90), this.modelMatrix);
  }
}

const BlurPass1 = {
  postProcess: true,
  beforeRender: (state, uniforms) => {
    uniforms.horizontal = true;
    uniforms.weights = GAUSS_WEIGHTS;
  },
  fragmentShader: require('./blur.glsl'),
};

const BlurPass2 = {
  postProcess: true,
  beforeRender: (state, uniforms) => {
    uniforms.horizontal = false;
    uniforms.weights = GAUSS_WEIGHTS;
  },
  fragmentShader: require('./blur.glsl'),
};

const MotionBlur = {
  postProcess: true,
  fragmentShader: require('./motionblur.glsl'),
};

const ScenePass = {
  postProcess: false,
  beforeRender: ({
    gl,
    width,
    height,
    camera,
    entities,
    entityProgramInfo,
    grungeTexture
  }) => {
    // Update the entities with some transforms
    entities.forEach((entity) => {
      twgl.m4.rotateY(entity.modelMatrix, 16 * 0.002, entity.modelMatrix);
    });

    // Inverse the cameras model matrix to create the view matrix
    const viewMatrix = twgl.m4.identity();
    twgl.m4.inverse(camera.modelMatrix, viewMatrix);

    // Then multiply by the projection matrix to get the view projection matrix
    const viewProjectionMatrix = twgl.m4.identity();
    twgl.m4.multiply(camera.projectionMatrix, viewMatrix, viewProjectionMatrix);

    // Render the sketch and update child texture from the hidden canvas
    // sketch.draw(state.childState, delta);
    // twgl.setTextureFromElement(gl, childTexture, childCanvas);
    gl.enable(gl.DEPTH_TEST);
    gl.viewport(0, 0, width, height);
    gl.useProgram(entityProgramInfo.program);

    const modelViewProjectionMatrix = twgl.m4.identity();

    entities.forEach((entity) => {
      twgl.m4.multiply(viewProjectionMatrix, entity.modelMatrix, modelViewProjectionMatrix);

      twgl.setUniforms(entityProgramInfo, {
        grungeTexture: grungeTexture,
        modelViewProjection: modelViewProjectionMatrix,
      });

      twgl.setBuffersAndAttributes(gl, entityProgramInfo, entity.bufferInfo);
      twgl.drawBufferInfo(gl, entity.bufferInfo);
    });
  },
};

const renderConfig = [
  ScenePass,
];

function setup (state) {
  const width = 1920 / 2;
  const height = 1080 / 2;

  state.canvas.style['display'] = 'none';
  const canvas = document.createElement('canvas');
  canvas.id = 'app';
  canvas.width = width;
  canvas.height = height;
  document.body.appendChild(canvas);

  const gl = canvas.getContext('webgl');

  /*
  const ecs = new EntityComponentSystem();
  const movementSystem = new MovementSystem(ecs);

  // const camera = new Camera(90, width / height, 0.1, 50)

  const camera = ecs.spawn(
    ProjectionComponent.perspective(45, width / height, 0.1, 50),
    TransformComponent.lookAt([0, 0, 5], [0, 0, 0], [0, 1, 0]),
  );

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

    const programInfo = (fragmentShader) ?
      twgl.createProgramInfo(gl, [
        vertexShader || VERTEX_SHADER,
        fragmentShader
      ]) : undefined;

    // Render to a frame buffer or the screen
    const inputTexture = frameBufferInfos[(i + 0) % 2].attachments[0];
    const outputFrameBuffer = (i === renderConfig.length - 1) ?
      undefined :
      frameBufferInfos[(i + 1) % 2];

    return {
      programInfo,
      inputTexture,
      outputFrameBuffer,
      beforeRender: config.beforeRender || function () {},
      postProcess: config.postProcess,
    };
  });

  // Move camera eye up and further back, looking at origin
  const camera = new Camera(90, width / height, 0.1, 50)
    .lookAt([0, 0, 5], [0, 0, 0], [0, 1, 0]);

  const entities = [
    new Cube(1, [ 1.5, -1.5,  0]),
    new Cube(1, [ 0,    0,    0]),
    new Cube(1, [-1.5,  1.5,  0]),
  ];

  entities.forEach((entity) => {
    entity.bufferInfo = twgl.createBufferInfoFromArrays(gl, entity.arrays);
  });

  const entityProgramInfo = twgl.createProgramInfo(gl, [
    entityVertexShader,
    entityFragmentShader
  ]);

  const grungeTexture = twgl.createTexture(gl, {
    src: '/renderer/grunge.jpg'
  });

  const s = {
    gl,
    width,
    height,
    t: 0,

    bufferInfo,
    frameBufferInfos,
    renderPasses,

    entities,
    camera,
    entityProgramInfo,
    grungeTexture,
  };

  return s;
  */

  const render = demo(gl, width, height);
  return {
    render
  };
}

function draw (state, delta) {
  state.render(delta);
  /*
  const {
    gl,
    frameBufferInfos,
    bufferInfo,
    width,
    height,
    renderPasses,
  } = state;

  state.t += delta;

  const uniforms = {
    time: state.t / 1000.0,
    resolution: [width, height],
  };

  gl.viewport(0, 0, width, height);
  renderPasses.forEach((renderPass, i) => {
    twgl.bindFramebufferInfo(gl, renderPass.outputFrameBuffer);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    uniforms.inputTexture = renderPass.inputTexture;

    renderPass.beforeRender(state, uniforms);
    gl.disable(gl.DEPTH_TEST);

    if (renderPass.postProcess !== false) {
      gl.useProgram(renderPass.programInfo.program);
      twgl.setBuffersAndAttributes(gl, renderPass.programInfo, bufferInfo);
      twgl.setUniforms(renderPass.programInfo, uniforms);
      twgl.drawBufferInfo(gl, bufferInfo);
    }
  });
  */
}

function handlers () {
  // if (!sketch.handlers) {
    return {};
  // }

  const child = sketch.handlers();
  return {
    '/note_on': function (msg, state) {
      child['/note_on'](msg, state.childState);
    },
  };
}

module.exports = {
  setup,
  draw,
  handlers,
};
