#version 300 es
// Raymarched liquid. sdBody/scene mirror World::sdf in sim.rs, plus cosmetic
// ripples and idle wobble that only exist here.
precision highp float;
// A zero the compiler can't see through. Loops start from it so they stay loops:
// unrolled, every scene() call would be copied dozens of times and some GPU
// drivers take minutes (or crash the browser) compiling the result.
#define ZERO min(uNH, 0)
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
// Slicing (meiosis.rs). Every value glides 0..1, so the shape never jumps.
uniform vec4 uCut;     // xyz normal of the cut, w split: 0 one body, 1 two cells
uniform vec3 uDiv;     // x flash of the fresh cut, y cleavage furrow, z cell material
// Past two (morula.rs): up to sixteen cells, then grain, then stone.
uniform vec4 uCells[16]; // xyz offset from the body center, w radius
uniform vec2 uCellF[16]; // x wounds 0..1, y death 0..1
uniform int uNC;         // 0 while it is one body or two cells
uniform vec4 uMor;       // x cells to grain, y grain count, z previous grain count, w previous to current
uniform float uGrainR;   // radius of the grain body, in body radii
uniform vec4 uStone;     // x stone, y risen into the slab, z thawed, w yaw the slab faces
uniform vec4 uSlab;      // xyz slab center, w smoothing between cells
uniform vec4 uEye;       // xyz where the eye looks, w how open
uniform vec4 uEyeC;      // xyz the eye cell's offset from the body center, w its radius
// Slamming (fusion.rs): the floor turns liquid, spreading from the impact.
uniform vec4 uLake;      // xy impact point on the floor (x, z), z liquid radius, w seconds since impact
uniform vec4 uLakeRip[6]; // x, z, age, amplitude of a touch on the lake
uniform int uNLR;
const float FLOOR_Y = -1.45;
// What the lake becomes (mere.rs), each gliding 0..1.
uniform vec4 uMere;      // x mirror, y deep, z ice
uniform vec4 uIce;       // xy where it froze from (x, z), z how far the ice reaches
uniform vec4 uCrack[6];  // x, z, age, seed of each blow to the ice
uniform int uNCrack;
uniform vec4 uJet;       // xy where the fountain stands (x, z), z column height, w drop height (< 0: none)
uniform vec2 uCoat;      // what it rose as: x quicksilver, y ink

// Bursting (burst.rs): twelve pieces that become twelve triangles.
uniform vec4 uBurst;     // x body gone, y flash of the pop, z wounds 0..1, w spread of the pieces 0..1
uniform vec4 uTri[12];   // xyz where the piece is (on the floor once landed), w height
uniform vec4 uTriB[12];  // x yaw, y blob→triangle morph, z wobble from a poke, w weathering (extra rounding)

// Each triangle's outline (base on y = 0, height 1), thickness, and edge rounding.
// Order matches Consistency in burst.rs: milk, chrome, glass, jelly, stone, honey,
// velvet, wax, ice, rubber, obsidian, moss.
const vec2 TA[12] = vec2[12](vec2(-0.5, 0.0), vec2(-0.25, 0.0), vec2(-0.4, 0.0), vec2(-0.6, 0.0), vec2(-0.6, 0.0), vec2(-0.45, 0.0),
                             vec2(-0.5, 0.0), vec2(-0.4, 0.0), vec2(-0.15, 0.0), vec2(-0.55, 0.0), vec2(-0.35, 0.0), vec2(-0.6, 0.0));
const vec2 TB[12] = vec2[12](vec2(0.5, 0.0), vec2(0.25, 0.0), vec2(0.45, 0.0), vec2(0.6, 0.0), vec2(0.4, 0.0), vec2(0.45, 0.0),
                             vec2(0.5, 0.0), vec2(0.4, 0.0), vec2(0.3, 0.0), vec2(0.55, 0.0), vec2(0.5, 0.0), vec2(0.6, 0.0));
const vec2 TC[12] = vec2[12](vec2(0.0, 0.87), vec2(0.0, 1.0), vec2(-0.4, 0.9), vec2(0.05, 0.55), vec2(0.55, 0.8), vec2(-0.1, 0.8),
                             vec2(0.3, 0.95), vec2(0.0, 0.7), vec2(-0.05, 1.0), vec2(0.0, 0.6), vec2(-0.2, 1.0), vec2(-0.3, 0.7));
const float TTHICK[12] = float[12](0.12, 0.08, 0.06, 0.25, 0.2, 0.15, 0.14, 0.3, 0.07, 0.22, 0.1, 0.18);
const float TROUND[12] = float[12](0.06, 0.01, 0.01, 0.12, 0.02, 0.08, 0.05, 0.1, 0.015, 0.1, 0.005, 0.04);
// Materials: base colour, then (gloss, reflectivity, translucency, grain).
const vec3 TCOL[12] = vec3[12](vec3(0.95, 0.94, 0.97), vec3(0.80, 0.82, 0.86), vec3(0.86, 0.96, 0.98), vec3(0.93, 0.22, 0.45),
                               vec3(0.62, 0.58, 0.53), vec3(0.96, 0.60, 0.14), vec3(0.16, 0.19, 0.56), vec3(0.96, 0.90, 0.78),
                               vec3(0.80, 0.91, 1.00), vec3(0.95, 0.36, 0.16), vec3(0.05, 0.05, 0.07), vec3(0.36, 0.55, 0.26));
const vec4 TMAT[12] = vec4[12](vec4(90.0, 0.25, 0.6, 0.0), vec4(300.0, 0.95, 0.0, 0.0), vec4(400.0, 0.6, 0.9, 0.0), vec4(60.0, 0.15, 0.9, 0.0),
                               vec4(8.0, 0.02, 0.0, 0.35), vec4(120.0, 0.2, 0.85, 0.0), vec4(6.0, 0.0, 0.1, 0.05), vec4(25.0, 0.05, 0.7, 0.03),
                               vec4(200.0, 0.35, 0.6, 0.12), vec4(15.0, 0.03, 0.05, 0.02), vec4(500.0, 0.55, 0.0, 0.0), vec4(5.0, 0.0, 0.2, 0.45));

const float SEP = 0.95;
const float PI = 3.14159265;
const float PHI = 1.61803399;
// Sides 1 : 4 : 9, holding the volume of a body of radius 1.
const float SLAB_UNIT = 0.48821;
const float CELL_SCALE = 0.7937;
const float K_JOIN = 0.9;

float smin(float a, float b, float k) {
  float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
  return mix(b, a, h) - k * h * (1.0 - h);
}

float sdTaper(vec3 p, vec3 a, vec3 b, float ra, float rb) {
  vec3 pa = p - a, ba = b - a;
  float h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
  return length(pa - ba * h) - mix(ra, rb, h);
}

// Ripples and wobble for a surface direction n, in body radii.
float surfaceDisp(vec3 n) {
  float disp = 0.012 * sin(n.x * 3.0 + uTime * 1.3) * sin(n.y * 4.0 - uTime * 1.1) * sin(n.z * 3.0 + uTime * 0.9);
  for (int i = ZERO; i < 8; i++) {
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
  // The cleavage furrow: a ring squeezed in around the cut, gone once they part.
  float c = dot(n, uCut.xyz);
  disp -= uDiv.y * (1.0 - uCut.w) * 0.32 * exp(-c * c * 9.0);
  return disp;
}

// Radius of each cell, and its center's offset from the body center (cell A at -offset).
float cellRadius() { return uBody.w * mix(1.0 - K_JOIN * 0.25, CELL_SCALE, uCut.w); }
vec3 cellOffset() { return uCut.xyz * (SEP * uBody.w * uCut.w); }

// The i-th of n points spread evenly over the unit sphere, and the nearest of
// them to p (spherical Fibonacci mapping, Keinert et al. 2015): four candidates
// instead of n, so a body of five hundred grains costs the same as one of thirty.
vec3 sfPoint(float i, float n) {
  float phi = 2.0 * PI * fract(i * PHI);
  float ct = 1.0 - 1.0 / n - 2.0 * i / n;
  float st = sqrt(max(1.0 - ct * ct, 0.0));
  return vec3(cos(phi) * st, sin(phi) * st, ct);
}

vec3 nearestSF(vec3 p, float n) {
  float m = 1.0 - 1.0 / n;
  float phi = min(atan(p.y, p.x), PI);
  float k = max(2.0, floor(log(max(n * PI * sqrt(5.0) * (1.0 - p.z * p.z), 1e-6)) / log(PHI * PHI)));
  float fk = pow(PHI, k) / sqrt(5.0);
  vec2 F = vec2(floor(fk + 0.5), floor(fk * PHI + 0.5));
  vec2 ka = 2.0 * F / n;
  vec2 kb = 2.0 * PI * (fract((F + 1.0) * PHI) - (PHI - 1.0));
  mat2 iB = mat2(ka.y, -ka.x, kb.y, -kb.x) / (ka.y * kb.x - ka.x * kb.y);
  vec2 c = floor(iB * vec2(phi, p.z - m));
  vec3 best = vec3(0.0, 0.0, 1.0);
  float d = 8.0;
  for (int s = ZERO; s < 4; s++) {
    float i = clamp(dot(F, vec2(s & 1, s >> 1) + c), 0.0, n - 1.0);
    vec3 q = sfPoint(i, n);
    float dq = dot(q - p, q - p);
    if (dq < d) { d = dq; best = q; }
  }
  return best;
}

// Height of the grain at direction n: a dome over each of `count` grains, creased between them.
float grainDome(vec3 n, float count) {
  float s = sqrt(4.0 * PI / count);
  float x = length(n - nearestSF(n, count)) / (0.58 * s);
  return 0.38 * s * sqrt(max(1.0 - x * x, 0.0));
}

float grainSurface(vec3 q) {
  float R = uGrainR * uBody.w;
  vec3 n = normalize(q + 1e-5);
  // the count after the cut and before it, blended, through one copy of the lookup
  float h = 0.0;
  for (int j = ZERO; j < 2; j++) {
    float w = j == 0 ? uMor.w : 1.0 - uMor.w;
    if (w < 0.001) continue;
    h += w * grainDome(n, j == 0 ? uMor.y : uMor.z);
  }
  // steep domes: step a little shorter over them
  return (length(q) - R * (1.0 + h * (1.0 - uStone.x))) * 0.8;
}

// The cells packed together, or the grain they became, around the body center.
// Ripples are added by bodySurface.
float clusterSurface(vec3 q) {
  float r = uBody.w;
  float lq = length(q);
  // nothing of it reaches much past two body radii (BOUND in morula.rs)
  if (uStone.y < 0.001 && lq > 2.5 * r) return lq - 2.3 * r;
  float d = 1e3;
  if (uMor.x < 0.999) {
    for (int i = ZERO; i < 16; i++) {
      if (i >= uNC) break;
      d = smin(d, length(q - uCells[i].xyz) - uCells[i].w, uSlab.w);
    }
  }
  float toBall = max(uMor.x, uStone.x * 0.8);
  if (toBall > 0.001) d = mix(d, uMor.x > 0.001 ? grainSurface(q) : lq - uGrainR * r, toBall);
  return d;
}

// An upright box standing on the floor, turned to face whoever made it.
float slabSurface(vec3 p) {
  vec3 q = p - uSlab.xyz;
  float c = cos(uStone.w), s = sin(uStone.w);
  q = vec3(c * q.x - s * q.z, q.y, s * q.x + c * q.z);
  vec3 h = vec3(2.0, 4.5, 0.5) * SLAB_UNIT * uBody.w;
  // its edges are only just softened, enough to catch the light
  float bevel = 0.06 * h.z;
  vec3 o = abs(q) - h + bevel;
  return length(max(o, 0.0)) + min(max(o.x, max(o.y, o.z)), 0.0) - bevel;
}

float bodySurface(vec3 p) {
  vec3 q = (p - uBody.xyz) / uScale;
  float lip = min(uScale.x, uScale.y);
  float r = uBody.w;
  float rc = cellRadius();
  vec3 off = cellOffset();
  // Still one body: both halves coincide, so evaluate one and add the join's bulge.
  bool one = uCut.w < 1e-4;
  // One pass per cell, so the compiler builds the ripples once: two cells, one
  // body, or (past two) everything around its center at once.
  float da = 0.0, db = 0.0;
  for (int i = ZERO; i < 2; i++) {
    vec3 v = i == 0 ? q + off : q - off;
    float base = length(v) - rc;
    float amt = 1.0;
    if (uNC > 0) {
      v = q;
      base = clusterSurface(q);
      // ripples, wobble and gaze run over the whole of it, less as it hardens
      amt = 0.6 * (1.0 - uStone.x);
    }
    vec3 n = normalize(v + 1e-5);
    // The twin's ripples are the first cell's, mirrored across the cut.
    if (i == 1) n = reflect(n, uCut.xyz);
    float di = base - surfaceDisp(n) * r * amt;
    if (i == 0) da = di; else db = di;
    if (one || uNC > 0) break;
  }
  if (uNC > 0) {
    float d = da * lip;
    if (uStone.y > 0.0) d = mix(d, mix(slabSurface(p), length(p - uBody.xyz) - uBody.w, uStone.z), uStone.y);
    return d;
  }
  if (one) return (da - K_JOIN * r * 0.25) * lip;
  return smin(da, db, max(K_JOIN * r * (1.0 - uCut.w), 1e-4)) * lip;
}

// How much of the floor at xz is liquid, 0..1.
float lakeMask(vec2 xz) { return smoothstep(uLake.z, uLake.z - 0.8, length(xz - uLake.xy)); }

// Height of the liquid floor: rings from the impact, rings from touches, a slow swell.
float lakeHeight(vec2 xz) {
  float m = lakeMask(xz);
  if (m <= 0.0) return 0.0;
  float dist = length(xz - uLake.xy);
  float t = uLake.w;
  float front = t * 2.4;
  float h = 0.10 * exp(-t * 0.5) * sin(dist * 5.0 - t * 8.0) * exp(-dist * 0.22)
            * smoothstep(front + 0.6, front - 0.6, dist);
  for (int i = ZERO; i < 6; i++) {
    if (i >= uNLR) break;
    float di = length(xz - uLakeRip[i].xy);
    float a = uLakeRip[i].z;
    h += 0.06 * uLakeRip[i].w * exp(-a * 0.8) * sin(di * 9.0 - a * 10.0) * exp(-di * 0.35)
         * smoothstep(a * 2.0 + 0.4, a * 2.0 - 0.3, di);
  }
  h += 0.012 * sin(xz.x * 1.3 + uTime * 0.9) * sin(xz.y * 1.1 - uTime * 0.7) * (1.0 - uMere.z);
  // a mirror is a lake with nothing moving in it
  return h * m * (1.0 - uMere.x);
}

float sdTri2(vec2 p, vec2 p0, vec2 p1, vec2 p2) {
  vec2 e0 = p1 - p0, e1 = p2 - p1, e2 = p0 - p2;
  vec2 v0 = p - p0, v1 = p - p1, v2 = p - p2;
  vec2 pq0 = v0 - e0 * clamp(dot(v0, e0) / dot(e0, e0), 0.0, 1.0);
  vec2 pq1 = v1 - e1 * clamp(dot(v1, e1) / dot(e1, e1), 0.0, 1.0);
  vec2 pq2 = v2 - e2 * clamp(dot(v2, e2) / dot(e2, e2), 0.0, 1.0);
  float s = sign(e0.x * e2.y - e0.y * e2.x);
  vec2 d = min(min(vec2(dot(pq0, pq0), s * (v0.x * e0.y - v0.y * e0.x)),
                   vec2(dot(pq1, pq1), s * (v1.x * e1.y - v1.y * e1.x))),
                   vec2(dot(pq2, pq2), s * (v2.x * e2.y - v2.y * e2.x)));
  return -sqrt(d.x) * sign(d.y);
}

// One piece: a liquid blob in flight that hardens into its triangle as it lands.
float shardSdf(vec3 p, int i) {
  vec3 base = uTri[i].xyz;
  float size = uTri[i].w;
  float m = uTriB[i].y;
  float blobR = 0.437 * uBody.w * mix(1.0, size * 0.5, m * m);
  float blob = length(p - base - vec3(0.0, blobR, 0.0)) - blobR;
  if (m <= 0.0) return blob;
  vec3 q = p - base;
  float yaw = uTriB[i].x + (1.0 - m) * 5.0;
  float c = cos(yaw), s = sin(yaw);
  q = vec3(c * q.x - s * q.z, q.y, s * q.x + c * q.z) / size;
  // a poke makes it wobble, more or less, by its consistency (burst.rs sets how much)
  q.x += uTriB[i].z * 0.07 * sin(q.y * 7.0 - uTime * 16.0) * q.y;
  float d2 = sdTri2(q.xy, TA[i], TB[i], TC[i]);
  float dz = abs(q.z) - TTHICK[i] * 0.5;
  // weather wears it down: the outline shrinks by as much as the edges round, so it keeps its size
  float w = uTriB[i].w;
  d2 += w; dz += w * 0.5;
  float tri = (min(max(d2, dz), 0.0) + length(max(vec2(d2, dz), 0.0)) - TROUND[i] - w) * size;
  // planted ones are cut off at the ground
  return max(mix(blob, tri, m), FLOOR_Y - p.y);
}

// Nearest piece and its distance.
float shards(vec3 p, out int id) {
  float d = 1e3;
  id = -1;
  for (int i = ZERO; i < 12; i++) {
    float di = shardSdf(p, i);
    if (di < d) { d = di; id = i; }
  }
  return d;
}

float scene(vec3 p) {
  float d = uBurst.x > 0.5 ? 1e3 : bodySurface(p);
  if (uBurst.w > 0.0) {
    int id;
    d = min(d, shards(p, id));
  }
  if (uLake.z > 0.0) {
    // the body melts into the floor through a smooth join
    float lake = (p.y - FLOOR_Y - lakeHeight(p.xz)) * 0.8;
    if (uJet.w > -0.3) {
      // the fountain: a column thrown up out of the lake, and the drop it lets go of
      vec3 jb = vec3(uJet.x, FLOOR_Y, uJet.y);
      float jet = sdTaper(p, jb - vec3(0.0, 0.3, 0.0), jb + vec3(0.0, uJet.z, 0.0), 0.2, 0.07);
      jet = min(jet, length(p - jb - vec3(0.0, uJet.w + 0.12, 0.0)) - 0.13);
      lake = smin(lake, jet, 0.18);
    }
    d = smin(d, lake, 0.45 * uBody.w);
  }
  for (int i = ZERO; i < 4; i++) {
    if (i >= uNH) break;
    float stem = sdTaper(p, uHA[i].xyz, uHB[i].xyz, uHA[i].w, uHB[i].w);
    float tendril = smin(stem, length(p - uHB[i].xyz) - uHBulb[i], 0.12 * uBody.w);
    d = smin(d, tendril, uK.x);
  }
  for (int i = ZERO; i < 12; i++) {
    if (i >= uND) break;
    d = smin(d, length(p - uDrop[i].xyz) - uDrop[i].w, uK.y);
  }
  return d;
}

// Color at a surface point: blended through the same smooth joins as the shape.
vec3 sceneColor(vec3 p) {
  float d = bodySurface(p);
  vec3 col = uBodyCol;
  for (int i = ZERO; i < 4; i++) {
    if (i >= uNH) break;
    float stem = sdTaper(p, uHA[i].xyz, uHB[i].xyz, uHA[i].w, uHB[i].w);
    d = smin(d, stem, uK.x);
  }
  for (int i = ZERO; i < 12; i++) {
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
  for (int layer = ZERO; layer < 2; layer++) {
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

// Tetrahedral gradient, as a loop so scene() is compiled once here, not four times.
vec3 calcNormal(vec3 p) {
  vec3 n = vec3(0.0);
  for (int i = ZERO; i < 4; i++) {
    vec3 k = 2.0 * vec3((((i + 3) >> 1) & 1), ((i >> 1) & 1), (i & 1)) - 1.0;
    n += k * scene(p + k * 0.0015);
  }
  return normalize(n);
}

// Soft white studio: bright from above, gray toward the floor, two softboxes.
vec3 env(vec3 r) {
  vec3 c = mix(vec3(0.70, 0.71, 0.74), vec3(1.0), smoothstep(-0.35, 0.55, r.y));
  c += vec3(1.4) * smoothstep(0.86, 0.97, dot(r, normalize(vec3(-0.5, 0.75, 0.45))));
  c += vec3(0.7) * smoothstep(0.90, 0.99, dot(r, normalize(vec3(0.8, 0.25, -0.55))));
  c *= 1.0 - 0.3 * exp(-abs(r.y + 0.05) * 9.0);
  return c;
}

float sphOcc(vec3 pos, vec4 sph) {
  vec3 di = sph.xyz - pos;
  float l = length(di);
  return max(0.0, di.y / l) * (sph.w * sph.w) / (l * l);
}

// The matte white floor at p, with the soft shadows of everything above it.
vec3 floorMatte(vec3 p, vec3 bg) {
  {
    vec3 off = cellOffset();
    float rs = mix(uBody.w, cellRadius(), uCut.w) * 1.1;
    float occ = uCut.w < 1e-4 ? sphOcc(p, vec4(uBody.xyz, uBody.w * 1.1))
                              : sphOcc(p, vec4(uBody.xyz - off, rs)) + sphOcc(p, vec4(uBody.xyz + off, rs));
    if (uNC > 0) {
      occ = mix(sphOcc(p, vec4(uBody.xyz, uBody.w * 1.25)),
                sphOcc(p, vec4(uSlab.x, FLOOR_Y + 0.7 * uBody.w, uSlab.z, 0.9 * uBody.w)), uStone.y);
    }
    for (int i = ZERO; i < 12; i++) {
      if (i >= uND) break;
      occ += sphOcc(p, uDrop[i]);
    }
    for (int i = ZERO; i < 4; i++) {
      if (i >= uNH) break;
      occ += sphOcc(p, vec4(uHB[i].xyz, uHBulb[i]));
    }
    if (uBurst.x > 0.5) occ = 0.0;
    float fade = exp(-0.05 * dot(p.xz, p.xz));
    // the pieces cast their own shadows wherever they stand
    float socc = 0.0;
    if (uBurst.w > 0.0) {
      for (int i = ZERO; i < 12; i++) {
        float h = uTri[i].w * mix(0.12, 0.4, uTriB[i].y);
        socc += sphOcc(p, vec4(uTri[i].xyz + vec3(0.0, h, 0.0), uTri[i].w * mix(0.12, 0.38, uTriB[i].y)));
      }
    }
    bg *= 1.0 - 0.55 * clamp(occ * fade + socc, 0.0, 1.0);
  }
  return bg;
}

vec3 shadeLiquid(vec3 p, vec3 rd);

// The liquid floor dissolves into the white with distance, so it has no far edge.
vec3 lakeFade(vec3 col, vec3 p, vec3 bg) {
  return mix(col, bg, smoothstep(6.0, 20.0, length(p - uCamPos)));
}

vec3 background(vec3 ro, vec3 rd, vec2 uv) {
  vec3 bg = vec3(0.972, 0.970, 0.962) - 0.03 * length(uv) * 0.5;
  if (rd.y < 0.0) {
    vec3 p = ro + rd * ((FLOOR_Y - ro.y) / rd.y);
    bg = floorMatte(p, bg);
    // past the marcher's reach the liquid floor still shows, flat
    if (uLake.z > 0.0) bg = mix(bg, lakeFade(shadeLiquid(p, rd), p, bg), lakeMask(p.xz));
  }
  if (uNight > 0.001) {
    vec3 sky = nightSky(rd);
    // Free drops glow like small suns at night.
    for (int i = ZERO; i < 12; i++) {
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


// Ray thickness through a sphere, starting at ro going along rd.
float sphereThickness(vec3 ro, vec3 rd, vec3 c, float r) {
  vec3 oc = ro - c;
  float b = dot(oc, rd);
  float disc = b * b - (dot(oc, oc) - r * r);
  if (disc <= 0.0) return 0.0;
  float s = sqrt(disc);
  return max(-b + s, 0.0) - max(-b - s, 0.0);
}

// Which cell p is on: its center and radius, a seed of its own, and its wounds and death.
void cellAt(vec3 p, out vec3 cc, out float rr, out float twin, out vec2 fate) {
  fate = vec2(0.0);
  if (uNC == 0) {
    twin = dot(p - uBody.xyz, uCut.xyz) > 0.0 ? 1.0 : 0.0;
    cc = uBody.xyz + cellOffset() * (twin * 2.0 - 1.0);
    rr = cellRadius();
    return;
  }
  vec3 q = p - uBody.xyz;
  if (uMor.x > 0.5) {
    vec3 c = nearestSF(normalize(q + 1e-5), uMor.y);
    float R = uGrainR * uBody.w;
    cc = uBody.xyz + c * R;
    rr = 0.5 * sqrt(4.0 * PI / uMor.y) * R;
    twin = fract(dot(c, vec3(17.1, 31.7, 7.3))) * 9.0;
    return;
  }
  float best = 1e3;
  for (int i = ZERO; i < 16; i++) {
    if (i >= uNC) break;
    float di = length(q - uCells[i].xyz) - uCells[i].w;
    if (di < best) {
      best = di;
      cc = uBody.xyz + uCells[i].xyz;
      rr = uCells[i].w;
      twin = float(i);
      fate = uCellF[i];
    }
  }
}

// A stained living cell: pale cytoplasm, a magenta membrane, and a violet
// nucleus with its chromatin, seen through the cell.
vec3 shadeCell(vec3 p, vec3 rd, vec3 n) {
  vec3 cc;
  float rr, twin;
  vec2 fate;
  cellAt(p, cc, rr, twin, fate);
  float ndv = clamp(dot(n, -rd), 0.0, 1.0);
  float rim = pow(1.0 - ndv, 2.2);
  vec3 col = mix(vec3(0.985, 0.925, 0.945), vec3(0.70, 0.27, 0.50), rim * 0.9);

  // the nucleus drifts slowly inside each cell
  vec3 nc = cc + 0.12 * rr * vec3(sin(uTime * 0.4 + twin * 3.0), cos(uTime * 0.33 + twin), sin(uTime * 0.27 + twin * 5.0));
  float nr = rr * 0.38;
  float th = sphereThickness(p, rd, nc, nr);
  vec3 mid = p + rd * dot(nc - p, rd);
  float grain = 0.6 + 0.4 * sin(dot(mid - nc, vec3(23.0, 17.0, 29.0)) / uBody.w + uTime * 0.6)
                          * sin(dot(mid - nc, vec3(-19.0, 31.0, 13.0)) / uBody.w);
  col *= mix(vec3(1.0), vec3(0.42, 0.24, 0.60), (1.0 - exp(-th / uBody.w * 3.2)) * (0.75 + 0.25 * grain));
  float tn = sphereThickness(p, rd, nc + vec3(0.06, 0.05, 0.0) * rr, nr * 0.32);
  col *= mix(vec3(1.0), vec3(0.32, 0.14, 0.42), 1.0 - exp(-tn / uBody.w * 6.0));

  // cytoplasm granules
  float g = 0.0;
  for (int i = ZERO; i < 6; i++) {
    float fi = float(i);
    vec3 gd = normalize(vec3(hash(vec3(fi, twin, 1.0)), hash(vec3(twin, fi, 2.0)), hash(vec3(2.0, fi, twin))) - 0.5);
    vec3 gp = cc + rr * 0.62 * gd + 0.06 * rr * vec3(sin(uTime * 0.7 + fi), cos(uTime * 0.5 + fi * 2.0), 0.0);
    g += sphereThickness(p, rd, gp, rr * 0.07);
  }
  col *= mix(vec3(1.0), vec3(0.78, 0.50, 0.66), 1.0 - exp(-g / uBody.w * 10.0));

  if (uNC > 0) {
    // a wounded cell flushes; a dying one empties to grey
    col *= mix(vec3(1.0), vec3(1.0, 0.70, 0.76), fate.x * 0.8);
    col = mix(col, vec3(0.60, 0.60, 0.62) * (0.9 + 0.2 * rim), smoothstep(0.0, 0.35, fate.y));
    // the newest cut glows through each cell's middle
    float seam = abs(dot(p - cc, uCut.xyz)) / max(rr, 1e-3);
    col += uDiv.x * vec3(1.0, 0.35, 0.65) * exp(-seam * 14.0);
    // The one that became different: an eye, looking where it looks.
    vec3 ev = p - uBody.xyz - uEyeC.xyz;
    float near = uEye.w * smoothstep(uEyeC.w * 1.45, uEyeC.w * 0.9, length(ev));
    if (near > 0.001) {
      // smaller on a body of grain than on a cell
      float iris = uMor.x > 0.5 ? 0.975 : 0.86;
      float a = (1.0 - dot(normalize(ev), uEye.xyz)) / (1.0 - iris);
      vec3 eye = mix(vec3(0.10, 0.03, 0.14), vec3(0.58, 0.26, 0.66), smoothstep(0.25, 1.0, a));
      vec3 side = normalize(cross(uEye.xyz, vec3(0.0, 1.0, 0.0)) + 1e-4);
      float spoke = atan(dot(normalize(ev), side), dot(normalize(ev), cross(uEye.xyz, side)));
      eye *= 0.8 + 0.2 * sin(spoke * 23.0 + twin) * smoothstep(0.3, 0.6, a);
      eye = mix(eye, vec3(0.0), smoothstep(0.45 - 0.15 * uEye.w, 0.3 - 0.15 * uEye.w, a));
      col = mix(col, eye, near * smoothstep(1.08, 0.95, a));
    }
  }

  vec3 L = normalize(vec3(-0.5, 0.8, 0.45));
  col *= 0.9 + 0.1 * max(dot(n, L), 0.0);
  vec3 r = reflect(rd, n);
  col += vec3(pow(max(dot(r, L), 0.0), 90.0) * 0.8);
  col += vec3(pow(max(dot(r, normalize(vec3(0.8, 0.25, -0.55))), 0.0), 40.0) * 0.12);
  return col;
}

// Polished black stone: almost nothing of its own, only what it reflects.
vec3 shadeStone(vec3 p, vec3 rd, vec3 n) {
  float ndv = clamp(dot(n, -rd), 0.0, 1.0);
  vec3 L = normalize(vec3(-0.5, 0.8, 0.45));
  vec3 col = vec3(0.016, 0.016, 0.02) * (0.5 + 0.5 * max(dot(n, L), 0.0));
  vec3 r = reflect(rd, n);
  float fres = 0.05 + 0.95 * pow(1.0 - ndv, 5.0);
  col += mix(env(r), nightSky(r) * 1.5, uNight) * fres * 0.8;
  col += vec3(pow(max(dot(r, L), 0.0), 240.0)) * 0.6;
  return col;
}

// One of the twelve, in its own material.
vec3 shadeShard(vec3 p, vec3 rd, int i) {
  vec3 n = calcNormal(p);
  vec3 v = -rd;
  float ndv = max(dot(n, v), 0.0);
  vec3 L = normalize(vec3(-0.5, 0.8, 0.45));
  vec3 base = TCOL[i];
  vec4 mat = TMAT[i];
  float wrap = mat.z;
  float diff = clamp((dot(n, L) + wrap) / (1.0 + wrap), 0.0, 1.0);
  vec3 col = base * (0.32 + 0.68 * diff);
  // light passing through the translucent ones, glowing at the thin edges
  col += base * mat.z * 0.35 * pow(1.0 - ndv, 1.5);
  if (mat.w > 0.0) col *= 1.0 - mat.w * hash(floor(p * 40.0));
  if (i == 6) col += base * 1.6 * pow(1.0 - ndv, 3.0); // velvet sheen
  if (i == 0) {
    vec3 irid = 0.5 + 0.5 * cos(6.2831 * ((1.0 - ndv) * 1.4 + uTime * 0.02 + vec3(0.0, 0.33, 0.67)));
    col = mix(col, irid, 0.3 * (1.0 - ndv));
  }
  float fres = mat.y + (1.0 - mat.y) * pow(1.0 - ndv, 5.0) * 0.7;
  col = mix(col, env(reflect(rd, n)) * (i == 1 ? base * 1.15 : vec3(1.0)), clamp(fres, 0.0, 1.0));
  col += vec3(pow(max(dot(reflect(-L, n), v), 0.0), mat.x)) * min(mat.x / 120.0, 1.0);
  return col;
}

vec3 shade(vec3 p, vec3 rd, vec2 uv) {
  int shard = -1;
  if (uBurst.w > 0.0) {
    int id;
    float ds = shards(p, id);
    float db = uBurst.x > 0.5 ? 1e3 : bodySurface(p);
    if (id >= 0 && ds < db) shard = id;
  }
  // One call site for shadeLiquid, so the compiler builds it once here.
  vec3 col = shard >= 0 || uDiv.z < 0.999 ? shadeLiquid(p, rd) : vec3(0.0);
  if (shard >= 0) {
    // in flight it is still the liquid; it takes on its material as it lands
    return mix(col, shadeShard(p, rd, shard), uTriB[shard].y);
  }
  // a body that has been hurt flushes
  col *= mix(vec3(1.0), vec3(1.0, 0.72, 0.78), uBurst.z * 0.9);
  if (uLake.z > 0.0) {
    // where the floor is hit but hasn't turned liquid yet, it is still the white floor
    float floorness = smoothstep(0.3, 0.08, p.y - FLOOR_Y) * (1.0 - lakeMask(p.xz));
    vec3 bg = vec3(0.972, 0.970, 0.962) - 0.03 * length(uv) * 0.5;
    float lakeness = smoothstep(0.3, 0.08, p.y - FLOOR_Y) * lakeMask(p.xz);
    col = mix(col, lakeFade(col, p, bg), lakeness);
    col = mix(col, floorMatte(p, bg), floorness);
  }
  if (uDiv.z > 0.001 || uStone.x > 0.001) {
    vec3 n = calcNormal(p);
    if (uDiv.z > 0.001) col = mix(col, shadeCell(p, rd, n), uDiv.z);
    if (uStone.x > 0.001) col = mix(col, shadeStone(p, rd, n), uStone.x);
  }
  if (uNC == 0) {
    // the cut: a hot seam where the slice plane crosses the surface
    float seam = abs(dot(p - uBody.xyz, uCut.xyz)) / uBody.w;
    col += uDiv.x * vec3(1.0, 0.35, 0.65) * (exp(-seam * 60.0) * 1.4 + exp(-seam * 10.0) * 0.12);
  }
  return col;
}

// The lake as mirror, deep, and ice. `col` is the plain liquid underneath.
vec3 mereSurface(vec3 p, vec3 rd, vec3 n, vec3 col, float fres) {
  vec3 r = reflect(rd, n);
  float ndv = max(dot(n, -rd), 0.0);
  if (uMere.x > 0.0) {
    // still: the room, perfectly, and the object standing in it where it fell
    vec3 mir = env(r) * vec3(0.93, 0.94, 0.96);
    vec3 gc = vec3(uLake.x, FLOOR_Y + uBody.w, uLake.y);
    vec3 oc = p - gc;
    float b = dot(oc, r);
    float disc = b * b - dot(oc, oc) + uBody.w * uBody.w;
    if (disc > 0.0 && -b - sqrt(disc) > 0.0) {
      vec3 gn = normalize(p + r * (-b - sqrt(disc)) - gc);
      float gd = max(dot(gn, -r), 0.0);
      vec3 ghost = mix(vec3(0.90, 0.91, 0.95) * (0.8 + 0.2 * gn.y), env(reflect(r, gn)), 0.15 + 0.85 * pow(1.0 - gd, 4.0));
      mir = mix(mir, ghost, 0.92);
    }
    col = mix(col, mir, uMere.x);
  }
  if (uMere.y > 0.0) {
    // deep: ink, and the object a faint moon far under it
    float dd = length(p.xz - uLake.xy);
    vec3 ink = vec3(0.015, 0.02, 0.04) + env(r) * fres * 0.7
             + vec3(0.85, 0.88, 1.0) * 0.4 * exp(-dd * dd * 0.6) * (0.8 + 0.2 * sin(uTime * 0.8));
    col = mix(col, ink, uMere.y);
  }
  float im = uMere.z * smoothstep(uIce.z, uIce.z - 0.6, length(p.xz - uIce.xy));
  if (im > 0.0) {
    // ice: frosted, pale, holding the shape it froze in, scored by every blow
    vec3 ice = mix(vec3(0.78, 0.86, 0.93), vec3(0.97, 0.99, 1.0), 0.5 + 0.5 * n.y);
    ice *= 0.93 + 0.07 * hash(floor(p * 55.0));
    ice = mix(ice, env(r), 0.2 + 0.6 * pow(1.0 - ndv, 4.0));
    float cr = 0.0;
    for (int i = ZERO; i < 6; i++) {
      if (i >= uNCrack) break;
      vec2 d = p.xz - uCrack[i].xy;
      float l = length(d);
      float reach = 2.4 * min(uCrack[i].z * 8.0, 1.0);
      float a = atan(d.y, d.x) * 7.0 / 6.2831 + uCrack[i].w * 7.0 + 0.06 * sin(l * 6.0 + uCrack[i].w * 40.0);
      float spoke = abs(fract(a) - 0.5) * 0.9 * l;
      cr += smoothstep(0.022, 0.004, spoke) * smoothstep(reach, reach * 0.4, l);
      cr += smoothstep(0.025, 0.0, abs(l - reach * 0.4)) * step(0.45, fract(a * 0.5 + uCrack[i].w));
    }
    ice = mix(ice, vec3(0.32, 0.40, 0.50), clamp(cr, 0.0, 1.0) * 0.8);
    col = mix(col, ice, im);
  }
  return col;
}

vec3 shadeLiquid(vec3 p, vec3 rd) {
  vec3 n = calcNormal(p);
  vec3 tint = sceneColor(p);
  vec3 v = -rd;
  float ndv = max(dot(n, v), 0.0);
  vec3 L = normalize(vec3(-0.5, 0.8, 0.45));

  // Cheap ambient occlusion from the field itself.
  float ao = 0.0;
  for (int i = ZERO + 1; i <= 4; i++) {
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

  // The lake's own states (mere.rs), and what the object rose as.
  float lk = uLake.z > 0.0 ? smoothstep(0.3, 0.08, p.y - FLOOR_Y) * lakeMask(p.xz) : 0.0;
  if (lk > 0.0) col = mix(col, mereSurface(p, rd, n, col, fres), lk);
  vec3 r = reflect(rd, n);
  col = mix(col, env(r) * vec3(0.86, 0.88, 0.92) + vec3(spec), uCoat.x * (1.0 - lk));
  col = mix(col, vec3(0.018, 0.02, 0.032) + tint * 0.03 + env(r) * fres * 0.8 + vec3(spec) * 0.6, uCoat.y * (1.0 - lk));

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
  // the landscape is far bigger than the room, so look further when it exists
  float tMax = 14.0 + 36.0 * uBurst.w;
  for (int i = ZERO; i < 140; i++) {
    vec3 p = ro + rd * t;
    float d = scene(p);
    if (d < 0.0012 * t) { hit = true; break; }
    t += d * 0.75;
    if (t > tMax) break;
  }

  vec3 col = hit ? shade(ro + rd * t, rd, uv) : background(ro, rd, uv);
  col = mix(col, vec3(1.0), uBurst.y);
  col = pow(col, vec3(0.97));
  outColor = vec4(col, 1.0);
}
