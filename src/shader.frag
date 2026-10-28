#version 300 es
// Raymarched liquid. sdBody/scene mirror World::sdf in sim.rs, plus cosmetic
// ripples and idle wobble that only exist here.
precision highp float;
out vec4 outColor;

uniform vec2 uRes;
uniform float uTime;
uniform vec3 uCamPos;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform vec3 uCamFwd;
uniform float uFocal;

uniform vec4 uBody;    // xyz center, w radius
uniform vec3 uScale;   // squash
uniform vec2 uK;       // smoothing: handles, drops
uniform vec4 uHA[4];   // tendril anchor xyz, base radius
uniform vec4 uHB[4];   // tendril tip xyz, neck radius
uniform float uHBulb[4];
uniform int uNH;
uniform vec4 uRip[8];  // ripple dir xyz, amplitude
uniform float uRipAge[8];
uniform int uNR;
uniform vec4 uDrop[12];
uniform int uND;

float smin(float a, float b, float k) {
  float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
  return mix(b, a, h) - k * h * (1.0 - h);
}

float sdTaper(vec3 p, vec3 a, vec3 b, float ra, float rb) {
  vec3 pa = p - a, ba = b - a;
  float h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
  return length(pa - ba * h) - mix(ra, rb, h);
}

float bodySurface(vec3 p) {
  vec3 q = p - uBody.xyz;
  float r = uBody.w;
  float d = (length(q / uScale) - r) * min(uScale.x, uScale.y);

  vec3 n = normalize(q + 1e-5);
  float disp = 0.012 * sin(n.x * 3.0 + uTime * 1.3) * sin(n.y * 4.0 - uTime * 1.1) * sin(n.z * 3.0 + uTime * 0.9);
  for (int i = 0; i < 8; i++) {
    if (i >= uNR) break;
    float a = acos(clamp(dot(n, uRip[i].xyz), -1.0, 1.0));
    float t = uRipAge[i];
    float A = uRip[i].w;
    // The dent where it was touched...
    disp -= 0.2 * A * exp(-a * a * 14.0) * exp(-7.0 * t) * (1.0 - exp(-60.0 * t));
    // ...and a wave that travels around the surface from it.
    float front = t * 1.5;
    float x = a - front;
    disp += 0.055 * A * exp(-1.3 * t) * cos(x * 12.0) * exp(-x * x * 5.0) * smoothstep(0.0, 0.25, t);
  }
  return d - disp * r;
}

float scene(vec3 p) {
  float d = bodySurface(p);
  for (int i = 0; i < 4; i++) {
    if (i >= uNH) break;
    float stem = sdTaper(p, uHA[i].xyz, uHB[i].xyz, uHA[i].w, uHB[i].w);
    float tendril = smin(stem, length(p - uHB[i].xyz) - uHBulb[i], 0.12 * uBody.w);
    d = smin(d, tendril, uK.x);
  }
  for (int i = 0; i < 12; i++) {
    if (i >= uND) break;
    d = smin(d, length(p - uDrop[i].xyz) - uDrop[i].w, uK.y);
  }
  return d;
}

vec3 calcNormal(vec3 p) {
  const vec2 k = vec2(1.0, -1.0);
  const float e = 0.0015;
  return normalize(k.xyy * scene(p + k.xyy * e) + k.yyx * scene(p + k.yyx * e) +
                   k.yxy * scene(p + k.yxy * e) + k.xxx * scene(p + k.xxx * e));
}

// Soft white studio: bright from above, gray toward the floor, two softboxes.
vec3 env(vec3 r) {
  vec3 c = mix(vec3(0.70, 0.71, 0.74), vec3(1.0), smoothstep(-0.35, 0.55, r.y));
  c += vec3(1.4) * smoothstep(0.86, 0.97, dot(r, normalize(vec3(-0.5, 0.75, 0.45))));
  c += vec3(0.7) * smoothstep(0.90, 0.99, dot(r, normalize(vec3(0.8, 0.25, -0.55))));
  c *= 1.0 - 0.3 * exp(-abs(r.y + 0.05) * 9.0);
  return c;
}

const float FLOOR_Y = -1.45;

float sphOcc(vec3 pos, vec4 sph) {
  vec3 di = sph.xyz - pos;
  float l = length(di);
  return max(0.0, di.y / l) * (sph.w * sph.w) / (l * l);
}

vec3 background(vec3 ro, vec3 rd, vec2 uv) {
  vec3 bg = vec3(0.972, 0.970, 0.962) - 0.03 * length(uv) * 0.5;
  if (rd.y < 0.0) {
    float t = (FLOOR_Y - ro.y) / rd.y;
    vec3 p = ro + rd * t;
    float occ = sphOcc(p, vec4(uBody.xyz, uBody.w * 1.1));
    for (int i = 0; i < 12; i++) {
      if (i >= uND) break;
      occ += sphOcc(p, uDrop[i]);
    }
    for (int i = 0; i < 4; i++) {
      if (i >= uNH) break;
      occ += sphOcc(p, vec4(uHB[i].xyz, uHBulb[i]));
    }
    float fade = exp(-0.05 * dot(p.xz, p.xz));
    bg *= 1.0 - 0.55 * clamp(occ, 0.0, 1.0) * fade;
  }
  return bg;
}

vec3 shade(vec3 p, vec3 rd) {
  vec3 n = calcNormal(p);
  vec3 v = -rd;
  float ndv = max(dot(n, v), 0.0);
  vec3 L = normalize(vec3(-0.5, 0.8, 0.45));

  // Cheap ambient occlusion from the field itself.
  float ao = 0.0;
  for (int i = 1; i <= 4; i++) {
    float h = 0.06 * float(i);
    ao += (h - scene(p + n * h)) / h;
  }
  ao = clamp(1.0 - 0.18 * ao, 0.0, 1.0);

  // Milky body with light scattering through it.
  float wrap = clamp((dot(n, L) + 0.6) / 1.6, 0.0, 1.0);
  vec3 deep = vec3(0.70, 0.74, 0.84);
  vec3 lit = vec3(0.97, 0.97, 0.99);
  vec3 col = mix(deep, lit, wrap) * mix(0.8, 1.0, ao);

  // Thin-film sheen at grazing angles, slowly drifting.
  float film = (1.0 - ndv);
  vec3 irid = 0.5 + 0.5 * cos(6.2831 * (film * 1.4 + n.y * 0.15 + uTime * 0.02 + vec3(0.0, 0.33, 0.67)));
  col = mix(col, irid * 0.95 + 0.1, 0.38 * film * film);

  // Glossy reflection.
  float fres = 0.04 + 0.96 * pow(1.0 - ndv, 5.0);
  vec3 refl = env(reflect(rd, n)) * ao;
  col = mix(col, refl, clamp(fres * 0.9 + 0.08, 0.0, 1.0));
  float spec = pow(max(dot(reflect(-L, n), v), 0.0), 90.0);
  col += vec3(0.9) * spec;
  return col;
}

void main() {
  vec2 uv = (2.0 * gl_FragCoord.xy - uRes) / uRes.y;
  vec3 ro = uCamPos;
  vec3 rd = normalize(uCamRight * uv.x + uCamUp * uv.y + uCamFwd * uFocal);

  float t = 0.0;
  bool hit = false;
  for (int i = 0; i < 110; i++) {
    vec3 p = ro + rd * t;
    float d = scene(p);
    if (d < 0.0012 * t) { hit = true; break; }
    t += d * 0.75;
    if (t > 14.0) break;
  }

  vec3 col = hit ? shade(ro + rd * t, rd) : background(ro, rd, uv);
  col = pow(col, vec3(0.97));
  outColor = vec4(col, 1.0);
}
