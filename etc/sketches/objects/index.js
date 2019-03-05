const twgl = require('twgl.js');

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

function setup (state) {
  const width = 1920 / 2;
  const height = 1080 / 2;

  state.canvas.style['display'] = 'none';
  const canvas = document.createElement('canvas');
  canvas.id = 'app';
  canvas.width = width;
  canvas.height = height;
  document.body.appendChild(canvas);

  // Move camera eye up and further back, looking at origin
  const camera = new Camera(90, width / height, 0.1, 50)
    .lookAt([0, 0, 5], [0, 0, 0], [0, 1, 0]);

  const entities = [
    new Cube(1, [ 1.5, -1.5,  0]),
    new Cube(1, [ 0,    0,    0]),
    new Cube(1, [-1.5,  1.5,  0]),
  ];

  const gl = canvas.getContext('webgl');

  entities.forEach((entity) => {
    entity.bufferInfo = twgl.createBufferInfoFromArrays(gl, entity.arrays);
  });

  const programInfo = twgl.createProgramInfo(gl, [
    entityVertexShader,
    entityFragmentShader
  ]);

  const grungeTexture = twgl.createTexture(gl, {
    src: '/objects/grunge.jpg'
  });

  return {
    gl,
    width,
    height,
    camera,
    entities,
    programInfo,
    grungeTexture,
    time: 0,
  };
}

function draw (state, delta) {
  const {
    gl,
    width,
    height,
    camera,
    entities,
    bufferInfo,
    programInfo,
    grungeTexture,
  } = state;

  state.time += delta;

  // twgl.m4.rotateY(camera.modelMatrix, delta * 0.002, camera.modelMatrix);
  // Update the cube with some transforms
  entities.forEach((entity) => {
    twgl.m4.rotateY(entity.modelMatrix, delta * 0.002, entity.modelMatrix);
  });

  // Inverse the cameras model matrix to create the view matrix
  const viewMatrix = twgl.m4.identity();
  twgl.m4.inverse(camera.modelMatrix, viewMatrix);

  // Then multiply by the projection matrix to get the view projection matrix
  const viewProjectionMatrix = twgl.m4.identity();
  twgl.m4.multiply(camera.projectionMatrix, viewMatrix, viewProjectionMatrix);

  const uniforms = {
    // modelViewProjection: modelViewProjectionMatrix,
    grungeTexture: grungeTexture,
  };

  gl.enable(gl.DEPTH_TEST);
  gl.viewport(0, 0, width, height);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

  gl.useProgram(programInfo.program);

  // Then multiply the entities model matrix to get a model view projection matrix
  // Obviously this needs to be done for each model in the scene (multiplying
  // their matrices by the view projection matrix)
  const modelViewProjectionMatrix = twgl.m4.identity();
  entities.forEach((entity) => {
    twgl.m4.multiply(viewProjectionMatrix, entity.modelMatrix, modelViewProjectionMatrix);
    uniforms.modelViewProjection = modelViewProjectionMatrix;

    twgl.setUniforms(programInfo, uniforms);
    twgl.setBuffersAndAttributes(gl, programInfo, entity.bufferInfo);
    twgl.drawBufferInfo(gl, entity.bufferInfo);
  });
}

module.exports = {
  setup,
  draw,
};
