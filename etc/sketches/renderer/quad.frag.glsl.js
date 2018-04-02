module.exports = `
precision mediump float;

uniform vec2 resolution;
uniform sampler2D inputTexture;

void main() {
	vec2 uv = gl_FragCoord.xy / resolution.xy;
  gl_FragColor = texture2D(inputTexture, uv);
}
`;
