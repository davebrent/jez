/*
https://extremeistan.wordpress.com/2014/09/24/physically-based-camera-rendering/
*/

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

varying vec3 vNormal;
varying vec2 vUv;

void main() {
  gl_FragColor = vec4(vUv.x, vUv.y, 1.0, 1.0);
}
`;

function radians (angle) {
  return angle * (Math.PI / 180);
}

class EntityComponentSystem {
  constructor() {
    this.ids = 0;
    this.components = {};
  }

  // Create a new entity with a set of components
  // Components must be instances of components rather than classes
  create(...components) {
    const entity = this.ids++;
    this.components[entity] = components.reduce((bundle, c) => {
      bundle[c.constructor.componentName] = c;
      return bundle;
    }, {});
    return entity;
  }

  // Destroy an entity
  destroy(entity) {
    delete this.components[entity];
  }

  // Return components from a specific entity
  queryOne(entity, ...querySet) {
    const components = this.components[entity];
    return querySet.reduce((result, query) => {
      const component = components[query.componentName];
      if (!component) {
        throw new Error('Component not found');
      }
      result.push(component);
      return result;
    }, []);
  }

  // Query entities with a set of components
  // Query set comprises of component classes rather than instances
  query(...querySet) {
    const result = [];
    const entries = Object.entries(this.components);

    for (let e = 0; e < entries.length; ++e) {
      const [entity, components] = entries[e];
      const row = [];

      for (let q = 0; q < querySet.length; ++q) {
        const component = components[querySet[q].componentName];
        if (!component) {
          break;
        }
        row.push(component);
      }

      if (row.length === querySet.length) {
        row.push(entity);
        result.push(row);
      }
    }

    return result;
  }
}

class PositionComponent {
  constructor(x, y, z) {
    this.vec = twgl.v3.create(x, y, z);
  }
}

class TransformComponent {
  constructor() {
    this.modelMatrix = twgl.m4.identity();
    this.modelViewProjectionMatrix = twgl.m4.identity();
  }

  static lookAt(eye, target, up) {
    const component = new TransformComponent();
    twgl.m4.lookAt(eye, target, up, component.modelMatrix);
    return component;
  }
}

class ProjectionComponent {
  constructor(fov, aspect, near, far) {
    this.projectionMatrix = twgl.m4.identity();
  }

  static perspective(fov, aspect, near, far) {
    const component = new ProjectionComponent();
    twgl.m4.perspective(
      radians(fov), aspect, near, far, component.projectionMatrix);
    return component;
  }
}

class MeshComponent {
  constructor(arrays) {
    this.arrays = arrays;
  }
}

class WaveModComponent {
  constructor(frequency, amplitude, phase) {
    this.frequency = frequency;
    this.amplitude = amplitude;
    this.phase = phase || 0;
  }
}

PositionComponent.componentName = 'position';
TransformComponent.componentName = 'transform';
ProjectionComponent.componentName = 'projection';
MeshComponent.componentName = 'mesh';

class TransformSystem {
  constructor(ecs) {
    this.ecs = ecs;
  }

  updateModelMatrices() {
    // Translate all entities, updating their model matrices
    this.ecs.query(
      PositionComponent,
      TransformComponent
    ).forEach(([position, transform, entity]) => {
      twgl.m4.setTranslation(
        transform.modelMatrix,
        position.vec,
        transform.modelMatrix);
    });
  }

  updateProjectionMatrices(camera) {
    // Get the required components for the camera entity
    const [cameraTransform, projection] = this.ecs.queryOne(
      camera,
      TransformComponent,
      ProjectionComponent);

    // Inverse the cameras model matrix to create the view matrix
    const viewMatrix = twgl.m4.inverse(cameraTransform.modelMatrix);

    // Then multiply by the projection matrix, to get the view projection matrix
    const viewProjectionMatrix = twgl.m4.multiply(
      projection.projectionMatrix,
      viewMatrix);

    // Calculate model view projection matrices for each entity
    this.ecs.query(
      TransformComponent
    ).forEach(([transform, entity]) => {
      twgl.m4.multiply(
        viewProjectionMatrix,
        transform.modelMatrix,
        transform.modelViewProjectionMatrix);
    });
  }
}

class WaveModSystem {
  constructor(ecs) {
    this.ecs = ecs;
    this.t = 0;
  }

  update(delta) {
    this.ecs.query(
      WaveModComponent,
      PositionComponent,
    ).forEach(([wave, position]) => {
      const {frequency, amplitude, phase} = wave;
      const sample = Math.sin(
        ((2 * Math.PI * frequency * this.t) + phase)
      ) * amplitude;
      position.vec[1] = sample;
    });
    this.t += (delta / 1000);
  }
}

class RenderSystem {
  constructor(ecs, gl, width, height) {
    this.ecs = ecs;
    this.gl = gl;
    this.width = width;
    this.height = height;

    this.bufferInfos = {};
    this.simpleProgram = twgl.createProgramInfo(this.gl, [
      entityVertexShader,
      entityFragmentShader
    ]);
  }

  render() {
    // Create buffers for all mesh components that havent been seen
    this.ecs.query(MeshComponent).forEach(([mesh, entity]) => {
      if (!this.bufferInfos[entity]) {
        this.bufferInfos[entity] = twgl.createBufferInfoFromArrays(
          this.gl, mesh.arrays);
      }
    });

    // Setup render states
    this.gl.enable(this.gl.DEPTH_TEST);
    this.gl.viewport(0, 0, this.width, this.height);
    this.gl.useProgram(this.simpleProgram.program);

    // Render entities
    this.ecs.query(
      TransformComponent,
      MeshComponent,
    ).forEach(([transform, mesh, entity]) => {
      twgl.setUniforms(this.simpleProgram, {
        modelViewProjection: transform.modelViewProjectionMatrix,
      });

      const bufferInfo = this.bufferInfos[entity];
      twgl.setBuffersAndAttributes(this.gl, this.simpleProgram, bufferInfo);
      twgl.drawBufferInfo(this.gl, bufferInfo);
    });
  }
}

function demo (gl, width, height) {
  const ecs = new EntityComponentSystem();

  const waveModSystem = new WaveModSystem(ecs);
  const transformSystem = new TransformSystem(ecs);
  const renderSystem = new RenderSystem(ecs, gl, width, height);

  // An a4 piece of paper
  const a4 = ecs.create(
    new PositionComponent(0, 0, 0),
    new TransformComponent(),
    new MeshComponent(twgl.primitives.createPlaneVertices(21, 29.7)),
    new WaveModComponent(1, 50, 0),
  );

  // const cameraMesh = ecs.create(
  //   new PositionComponent(0, 100, 0),
  //   new TransformComponent(),
  //   new MeshComponent(twgl.primitives.createCubeVertices(20, 20, 20)),
  // );
  //
  // const debugCamera = ecs.create(
  //   new PositionComponent(0, 100, 150),
  //   ProjectionComponent.perspective(45, width / height, 1, 5000),
  //   TransformComponent.lookAt(
  //     [0, 100, 150], // eye
  //     [0, 50, 0],   // target
  //     [0, 1, 0]   // up
  //   ),
  // );

  // 1 meter above the origin looking down on the a4 piece of paper
  // Cameras visible region covers 1cm - 50m
  const camera = ecs.create(
    new PositionComponent(0, 100, 0),
    ProjectionComponent.perspective(45, width / height, 1, 5000),
    TransformComponent.lookAt(
      [0, 100, 0], // eye
      [0, 0, 0],   // target
      [0, 0, -1]   // up
    ),
  );

  return (delta) => {
    waveModSystem.update(delta);
    transformSystem.updateModelMatrices();
    transformSystem.updateProjectionMatrices(camera);
    renderSystem.render();
  };
}

module.exports = {
  EntityComponentSystem,

  PositionComponent,
  TransformComponent,
  ProjectionComponent,
  MeshComponent,

  RenderSystem,
  TransformSystem,

  demo,
};
