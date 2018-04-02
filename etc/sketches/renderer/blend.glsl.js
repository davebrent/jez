// Combine an input image with the original image
module.exports = `
precision mediump float;

uniform sampler2D inputTexture;
uniform sampler2D originalImage;
uniform vec2 resolution;
uniform float contribution;

void main() {
	vec2 tc = gl_FragCoord.xy / resolution.xy;
  vec4 inputImage = texture2D(inputTexture, tc) * contribution;
  vec4 original = texture2D(originalImage, tc);
  vec3 result = inputImage.rgb + original.rgb;

  const float gamma = 2.2;
  result = pow(result.rgb, vec3(1.0 / gamma));
  gl_FragColor = vec4(result, 1.0);
}
`;
