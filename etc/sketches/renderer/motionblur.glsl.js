module.exports = `
precision mediump float;

uniform sampler2D inputTexture;
uniform sampler2D depthTexture;

uniform vec2 resolution;

void main() {
  // Current fragments texture coordinates
	vec2 tc = gl_FragCoord.xy / resolution.xy;

/*
  // Get the depth buffer value at this pixel.
  float zOverW = texture2D(depthTexture, tc);

  // H is the viewport position at this pixel in the range -1 to 1.
  vec4 H = float4(tc.x * 2 - 1, (1 - tc.y) * 2 - 1, zOverW, 1);

  // Transform by the view-projection inverse.
  vec4 D = mul(H, g_ViewProjectionInverseMatrix);

  // Divide by w to get the world position.
  vec4 worldPos = D / D.w;

  // Current viewport position
  vec4 currentPos = H;

  // Use the world position, and transform by the previous view-
  // projection matrix.
  vec4 previousPos = mul(worldPos, g_previousViewProjectionMatrix);

  // Convert to nonhomogeneous points [-1,1] by dividing by w.
  previousPos /= previousPos.w;

  // Use this frame's position and last frame's to compute the pixel
  // velocity.
  vec2 velocity = (currentPos - previousPos)/2.f;
*/
  vec2 velocity = vec2(-0.001, -0.001);

  // Get the initial color at this pixel.
  vec4 color = texture2D(inputTexture, tc);
  int numSamples = 12;

  for (int i = 1; i < 12; ++i) {
    // Sample the color buffer along the velocity vector.
    vec4 currentColor = texture2D(inputTexture, tc + (velocity * float(i)));
    // Add the current color to our color sum.
    color += currentColor;
  }

  // Average all of the samples to get the final blur color.
  gl_FragColor = color / float(12);
}
`;
