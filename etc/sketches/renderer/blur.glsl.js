module.exports = `
precision mediump float;

uniform sampler2D inputTexture;
uniform vec2 resolution;

uniform bool horizontal;
uniform float weights[5];

void main() {
  // Size of a single texel
  vec2 offset = 1.0 / resolution;

  // Current fragments texture coordinates
	vec2 tc = gl_FragCoord.xy / resolution.xy;

  // Current fragment's contribution
  vec3 result = texture2D(inputTexture, tc).rgb * weights[0];

  if (horizontal) {
    for(int i = 1; i < 5; ++i) {
      float f = float(i);
      result += texture2D(inputTexture, tc + vec2(offset.x * f, 0.0)).rgb * weights[i];
      result += texture2D(inputTexture, tc - vec2(offset.x * f, 0.0)).rgb * weights[i];
    }
  } else {
    for(int i = 1; i < 5; ++i) {
      float f = float(i);
      result += texture2D(inputTexture, tc + vec2(0.0, offset.y * f)).rgb * weights[i];
      result += texture2D(inputTexture, tc - vec2(0.0, offset.y * f)).rgb * weights[i];
    }
  }

  gl_FragColor = vec4(result, 1.0);
}
`;
