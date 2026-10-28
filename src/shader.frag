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

uniform vec3 uBodyCol;
uniform vec3 uDropCol[12];
uniform float uAgit;   // 0..1 how upset it is
uniform float uNight;  // 0..1 day to night
uniform vec4 uGaze;    // xyz direction it reaches toward, w amount

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
  // Watching: a slow swell toward the viewer.
  float g = acos(clamp(dot(n, uGaze.xyz), -1.0, 1.0));
  disp += uGaze.w * (0.30 * exp(-g * g * 4.0) + 0.01 * sin(uTime * 2.1) * exp(-g * g * 2.0));
  // Agitation: the surface boils.
  float t = uTime;
  disp += uAgit * 0.045 * sin(n.x * 17.0 + t * 23.0) * sin(n.y * 15.0 - t * 19.0) * sin(n.z * 16.0 + t * 21.0);
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

// Color at a surface point: blended through the same smooth joins as the shape.
vec3 sceneColor(vec3 p) {
  float d = bodySurface(p);
  vec3 col = uBodyCol;
  for (int i = 0; i < 4; i++) {
    if (i >= uNH) break;
    float stem = sdTaper(p, uHA[i].xyz, uHB[i].xyz, uHA[i].w, uHB[i].w);
    d = smin(d, stem, uK.x);
  }
  for (int i = 0; i < 12; i++) {
    if (i >= uND) break;
    float dd = length(p - uDrop[i].xyz) - uDrop[i].w;
    float h = clamp(0.5 + 0.5 * (d - dd) / uK.y, 0.0, 1.0);
    col = mix(col, uDropCol[i], h);
    d = smin(d, dd, uK.y);
  }
  return col;
}

float hash(vec3 p) {
  p = fract(p * 0.3183099 + 0.1);
  p *= 17.0;
  return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

// Stars on the inside of a sphere around everything.
vec3 stars(vec3 rd) {
  vec3 col = vec3(0.0);
  for (int layer = 0; layer < 2; layer++) {
    float scale = layer == 0 ? 60.0 : 140.0;
    vec3 q = rd * scale;
    vec3 id = floor(q);
    float h = hash(id + float(layer) * 31.0);
    if (h > 0.93) {
      vec3 c = id + 0.5 + 0.35 * (vec3(hash(id + 1.0), hash(id + 2.0), hash(id + 3.0)) - 0.5);
      float s = smoothstep(0.35, 0.0, length(q - c));
      float tw = 0.6 + 0.4 * sin(uTime * (1.0 + 3.0 * h) + h * 50.0);
      col += s * tw * mix(vec3(0.7, 0.8, 1.0), vec3(1.0, 0.85, 0.7), hash(id + 7.0)) * (layer == 0 ? 1.0 : 0.6);
    }
  }
  // A faint band of something far away.
  float band = exp(-pow(dot(rd, normalize(vec3(0.3, 1.0, -0.2))) * 3.5, 2.0));
  col += band * vec3(0.10, 0.08, 0.16) * (0.6 + 0.4 * hash(floor(rd * 400.0)));
  return col;
}

vec3 nightSky(vec3 rd) {
  return vec3(0.010, 0.012, 0.028) + stars(rd);
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
  if (uNight > 0.001) {
    vec3 sky = nightSky(rd);
    // Free drops glow like small suns at night.
    for (int i = 0; i < 12; i++) {
      if (i >= uND) break;
      vec3 c = uDrop[i].xyz - ro;
      float along = max(dot(c, rd), 0.0);
      float dist = length(c - rd * along);
      sky += uDropCol[i] * 0.006 * uDrop[i].w / (dist * dist + 0.004);
    }
    bg = mix(bg, sky, uNight);
  }
  return bg;
}

vec3 shade(vec3 p, vec3 rd) {
  vec3 n = calcNormal(p);
  vec3 tint = sceneColor(p);
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
  vec3 col = mix(deep * tint, lit * tint, wrap) * mix(0.8, 1.0, ao);
  // Upset, it darkens and flushes.
  col = mix(col, col * vec3(0.75, 0.45, 0.5), uAgit * 0.6);

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

  if (uNight > 0.001) {
    // At night it becomes a lens: the sky seen through it, a rim of its own color.
    vec3 inside = nightSky(refract(rd, n, 0.75)) * 1.6 + tint * 0.05;
    float rim = pow(1.0 - ndv, 3.0);
    vec3 night = inside + tint * rim * 0.9 + nightSky(reflect(rd, n)) * fres + vec3(0.8, 0.85, 1.0) * spec * 0.4;
    col = mix(col, night, uNight);
  }
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
