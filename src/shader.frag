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
// The river they run as when crowded together (river.rs), and the columns that stand out of it.
uniform vec4 uRiver;     // xy where it rose (x, z), z which way it runs, w how far it reaches either way (0: none)
uniform vec4 uRiver2;    // x how much river 0..1, y how far it has flowed, z its heartbeat 0..1, w the tallest column
uniform vec4 uPil[18];   // each column's height, four to a vec4: row by row along the river, three across
uniform vec4 uPilRow[6]; // for each row, rows to the nearest one with a column standing: ahead + 32 * behind
uniform vec4 uSplash[6]; // x, z, age of each place a column fell back in
uniform int uNSp;
const float PIL_GAP = 1.6;
const float PIL_HALF = 0.58;
const vec3 BLOOD = vec3(0.42, 0.02, 0.05);

// The other forms (forms/). Each is gated on its own amount, so it costs nothing until it is used.
uniform vec4 uBloom;     // x how open the flower is 0..1, y where the first petal points
uniform vec4 uBud[5];    // xyz where a pinch left a bud, w how grown it is
uniform int uNB;
uniform vec4 uFil;       // xyz which way the thread runs, w half its length
uniform vec4 uFil2;      // x its thickness, y sphere→thread 0..1, z ringing now, w which harmonic
uniform vec3 uFilSwing;  // which way it swings when plucked
uniform vec4 uCosmos;    // x liquid→universe 0..1, y how far the galaxy has turned, z a star igniting, w which way it turns
uniform vec4 uBox;       // x round→box 0..1, y the lid open 0..1, z rattle of a knock, w which way it faces
uniform vec2 uShock;     // x how hard the last shock was 0..1, y seconds since it (camera.rs)
uniform float uFine;     // 1 while frames keep up; 0 leaves out the light bent through it and focused under it

// ---- the other one (other.rs): its shape -------------------------------------
uniform vec4 uOther;     // xyz where it is, w its size (0: not in the room)
uniform vec4 uOther2;    // x how far it has turned, y how far it leans, z how sharp it has become, w ringing
uniform vec4 uOther3;    // rgb the color it kept from inside the first, a fire in its edges
uniform float uOtherIn;  // 0..1 how far inside the first it is

// ---- an old hurt (wound.rs) ------------------------------------------------------
uniform vec4 uScar;      // xyz which way the seam faces from the middle, w how plain the scar is (0: none)
uniform vec4 uWound;     // x how open 0..1, y how full 0..1, z a drip going in 1..0
uniform vec3 uWoundCol;  // what it is filling with
uniform int uCaltrop;    // which drops are its spiked pieces, one bit each
const int OTHER = 40;

// Turns that depend only on uniforms, worked out once per pixel by sceneSetup() so that
// scene() and the shading need not work them out again at every step.
vec4 gOtherTurn; // cos and sin of the other one's turn, then of its lean
vec2 gRiverDir;  // which way the river runs
vec4 gBoxTurn;   // cos and sin of the way the box faces, then of how far its lid is open
float gBoxShiver;
vec3 gScarS, gScarW; // along the seam, and across it

void sceneSetup() {
  vec3 ref = abs(uScar.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
  gScarS = normalize(cross(uScar.xyz, ref));
  gScarW = cross(uScar.xyz, gScarS);
  gOtherTurn = vec4(cos(uOther2.x), sin(uOther2.x), cos(uOther2.y), sin(uOther2.y));
  gRiverDir = vec2(cos(uRiver.z), sin(uRiver.z));
  float lid = max(uBox.y, 0.0) * 1.9;
  gBoxTurn = vec4(cos(uBox.w), sin(uBox.w), cos(lid), sin(lid));
  gBoxShiver = uBox.z * 0.025 * uBody.w * sin(uTime * 70.0);
}

// Long and many-sided: two eight-sided cuts at once, sixteen faces, a point at each end.
// Returns both cuts in its own units; where they are equal is an edge.
vec2 otherCuts(vec3 p) {
  vec3 q = (p - uOther.xyz) / uOther.w;
  vec4 cs = gOtherTurn;
  q.xz = mat2(cs.x, cs.y, -cs.y, cs.x) * q.xz;
  q.xy = mat2(cs.z, cs.w, -cs.w, cs.z) * q.xy;
  q.y /= 1.6 + 0.6 * uOther2.z;
  vec3 a = abs(q);
  vec3 b = vec3(abs(q.x + q.z) * 0.70711, a.y, abs(q.x - q.z) * 0.70711);
  return vec2(a.x + a.y + a.z, b.x + b.y + b.z) - 1.0;
}

float otherSdf(vec3 p) {
  vec2 k = otherCuts(p);
  return max(k.x, k.y) * 0.57735 * uOther.w;
}
// ---- end of the other one's shape ---------------------------------------------
const float HALF_SIDE = 0.806;

// ---- the fold (forms/fold.rs) ---------------------------------------------------------------
// Stirred on past its universe, what it holds gets out: the floor goes, the room folds into
// mirrors, and it is repeated in every direction, each copy the mirror of its neighbours.
// Everything here is a change of coordinates or a colour, so it costs scene() one transform and
// nothing at all until uFold.x leaves zero.
uniform vec4 uFold;      // x room→folded 0..1, y its own clock (runs back while held), z a new mirror's flash, w running backward 0..1
uniform vec4 uFold2;     // x mirrors before, y mirrors now, z before→now 0..1

vec2 foldRot(vec2 v, float a) {
  float c = cos(a), s = sin(a);
  return vec2(c * v.x - s * v.y, s * v.x + c * v.y);
}

// Its colours, run out: the galaxy's blue and pink, its core's gold, and the dark between.
vec3 foldPal(float t) {
  return vec3(0.42, 0.30, 0.55) + vec3(0.48, 0.42, 0.45) * cos(6.2831 * (vec3(1.0, 1.0, 1.0) * t + vec3(0.0, 0.36, 0.62)));
}

// Space around it, folded: turned on the fold's clock, twisted more the further out, then
// repeated on a lattice with every other cell mirrored. Copies near the eye shrink out of the
// way, so it is never inside one. Returns the point in its own copy's frame; `sgn`, `ang` say
// how that copy is flipped and turned (for directions), `k` which copy it is, `s` its scale.
vec3 foldSpace(vec3 p, out vec3 sgn, out float ang, out vec3 k, out float s) {
  sgn = vec3(1.0);
  ang = 0.0;
  k = vec3(0.0);
  s = 1.0;
  if (uFold.x <= 0.0) return p;
  float f = smoothstep(0.0, 1.0, uFold.x);
  vec3 q = p - uBody.xyz;
  ang = f * (uFold.y * 0.06 + 0.035 * length(q));
  // one turn, used both ways: space into the copies' frame, and the eye along with it
  float c = cos(ang), sn = sin(ang);
  q.xz = vec2(c * q.x - sn * q.z, sn * q.x + c * q.z);
  // the copies come in from far off, at a steady pace to the eye
  float S = uBody.w * 5.2 * exp(2.6 * (1.0 - f));
  k = floor(q / S + 0.5);
  q -= S * k;
  sgn = 1.0 - 2.0 * mod(k, 2.0);
  q *= sgn;
  if (dot(k, k) > 0.0) {
    // where this copy stands, and how far it is from the line between the eye and the one in
    // the middle: the ones in the way, and the ones too close, make room. Measured in the
    // turned frame, so the copy need not be turned back.
    vec3 w = S * k;
    vec3 e = uCamPos - uBody.xyz;
    e.xz = vec2(c * e.x - sn * e.z, sn * e.x + c * e.z);
    float off = length(w - e * clamp(dot(w, e) / dot(e, e), 0.0, 1.2));
    s = max(min(smoothstep(1.3 * uBody.w, 3.2 * uBody.w, off), smoothstep(1.5 * uBody.w, 4.0 * uBody.w, length(w - e))), 1e-3);
  }
  return uBody.xyz + q / s;
}

// How far a step may go through the turned space. The turn grows with the distance from the
// vertical through it, and stretches space by at most sqrt(1 + h * h) + h, where h is half the
// turn per unit times that distance. Never a shorter step than the fixed 0.7 it replaces.
float foldStep(vec3 p) {
  float h = 0.0175 * smoothstep(0.0, 1.0, uFold.x) * length(p.xz - uBody.xz);
  return max(sqrt(1.0 + h * h) - h, 0.7);
}

// The sky the room folds into: one sky, cut into mirrored wedges around the vertical and
// across the horizon, with an orbit-trap fractal in each, drifting on the fold's clock.
vec3 foldSky(vec3 rd) {
  float clk = uFold.y;
  // a new mirror count opens out of the old one; the flash covers the seam while it is between
  float n = mix(uFold2.x, uFold2.y, uFold2.z);
  float sec = 6.2831 / n;
  float a = mod(atan(rd.z, rd.x) + clk * 0.025, sec);
  a = min(a, sec - a);
  float h = length(rd.xz);
  vec3 q = vec3(cos(a) * h, abs(rd.y), sin(a) * h) * 1.15 + vec3(0.11, 0.07 * sin(clk * 0.05), 0.2);
  vec3 C = vec3(0.56, 0.38, 0.66) + 0.05 * vec3(sin(clk * 0.071), cos(clk * 0.053), sin(clk * 0.037));
  float last = 0.0;
  vec3 col = vec3(0.0);
  for (int i = ZERO; i < 11; i++) {
    q = abs(q) / dot(q, q) - C;
    float m = length(q);
    float e = exp(-abs(m - last) * 3.0);
    col += foldPal(float(i) * 0.07 + m * 0.22 - clk * 0.012) * e * e * 0.26;
    last = m;
  }
  // the seams of the mirrors catch the light
  float seam = exp(-a * h * 90.0) + exp(-abs(rd.y) * 70.0);
  col += vec3(0.95, 0.85, 1.0) * seam * (0.12 + 0.9 * uFold.z);
  col = vec3(0.03, 0.015, 0.07) + col * col * 1.6 * (1.0 + 0.8 * uFold.z);
  // running backward, the light goes over to the far side of its colours
  return mix(col, col.zxy * vec3(0.8, 1.05, 1.25), 0.55 * uFold.w);
}
// ---- end of the fold -------------------------------------------------------------------------

// Each triangle's outline: its base on y = 0 runs from x = TRI.x to TRI.y, its apex is at
// TRI.zw (height about 1); then its thickness and edge rounding.
// Order matches Consistency in burst.rs: milk, chrome, glass, jelly, stone, honey,
// velvet, wax, ice, rubber, obsidian, moss.
const vec4 TRI[12] = vec4[12](vec4(-0.5, 0.5, 0.0, 0.87), vec4(-0.25, 0.25, 0.0, 1.0), vec4(-0.4, 0.45, -0.4, 0.9), vec4(-0.6, 0.6, 0.05, 0.55),
                             vec4(-0.6, 0.4, 0.55, 0.8), vec4(-0.45, 0.45, -0.1, 0.8), vec4(-0.5, 0.5, 0.3, 0.95), vec4(-0.4, 0.4, 0.0, 0.7),
                             vec4(-0.15, 0.3, -0.05, 1.0), vec4(-0.55, 0.55, 0.0, 0.6), vec4(-0.35, 0.5, -0.2, 1.0), vec4(-0.6, 0.6, -0.3, 0.7));
const vec2 TSHAPE[12] = vec2[12](vec2(0.12, 0.06), vec2(0.08, 0.01), vec2(0.06, 0.01), vec2(0.25, 0.12), vec2(0.2, 0.02), vec2(0.15, 0.08),
                                 vec2(0.14, 0.05), vec2(0.3, 0.1), vec2(0.07, 0.015), vec2(0.22, 0.1), vec2(0.1, 0.005), vec2(0.18, 0.04));
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

// The scar at a surface direction n: x on the seam itself, y in the open wound, both 0..1.
vec2 scarAt(vec3 n) {
  if (uScar.w <= 0.0) return vec2(0.0);
  float along = dot(n, gScarS), across = dot(n, gScarW);
  float m = exp(-along * along * 14.0) * step(0.0, dot(n, uScar.xyz));
  return vec2(uScar.w * exp(-across * across * 300.0), uWound.x * exp(-across * across * 110.0)) * m;
}

// Ripples and wobble for a surface direction n, in body radii.
float surfaceDisp(vec3 n) {
  float disp = 0.012 * sin(n.x * 3.0 + uTime * 1.3) * sin(n.y * 4.0 - uTime * 1.1) * sin(n.z * 3.0 + uTime * 0.9);
  // Ripples and buds share one loop.
  for (int i = ZERO; i < 8; i++) {
    if (i >= uNR && i >= uNB) break;
    if (i < uNR) {
      float a = acos(clamp(dot(n, uRip[i].xyz), -1.0, 1.0));
      float t = uRipAge[i];
      float A = uRip[i].w;
      // The dent where it was touched...
      disp -= 0.2 * A * exp(-a * a * 14.0 - 7.0 * t) * (1.0 - exp(-60.0 * t));
      // ...and a wave that travels around the surface from it.
      float x = a - t * 1.5;
      disp += 0.055 * A * cos(x * 12.0) * exp(-1.3 * t - x * x * 5.0) * smoothstep(0.0, 0.25, t);
    }
    // Buds: a pinch leaves a small sharp peak.
    if (i < uNB) {
      vec4 b = uBud[min(i, 4)];
      disp += 0.3 * b.w * exp((dot(n, b.xyz) - 1.0) * 40.0);
    }
  }
  // Watching: a slow swell toward the viewer.
  float g = acos(clamp(dot(n, uGaze.xyz), -1.0, 1.0));
  float e = exp(-g * g * 2.0);
  disp += uGaze.w * e * (0.30 * e + 0.01 * sin(uTime * 2.1));
  // Agitation: the surface boils.
  float t = uTime;
  disp += uAgit * 0.045 * sin(n.x * 17.0 + t * 23.0) * sin(n.y * 15.0 - t * 19.0) * sin(n.z * 16.0 + t * 21.0);
  // The cleavage furrow: a ring squeezed in around the cut, gone once they part.
  float c = dot(n, uCut.xyz);
  disp -= uDiv.y * (1.0 - uCut.w) * 0.32 * exp(-c * c * 9.0);
  // The scar is raised a little; the wound is a gash, shallower as it fills.
  vec2 sc = scarAt(n);
  disp += 0.03 * sc.x - sc.y * (0.11 * (1.0 - 0.85 * uWound.y) + 0.02 * uWound.z * sin(uTime * 9.0));
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

float cellsSurface(vec3 p) {
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

float sdEllipsoid(vec3 p, vec3 r) {
  float k0 = length(p / r);
  float k1 = length(p / (r * r));
  return k0 * (k0 - 1.0) / k1;
}

float sdBox(vec3 p, vec3 b, float r) {
  vec3 q = abs(p) - b + r;
  return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

// The flower, facing up: a low heart and five petals that unfold from standing to open.
float flowerSdf(vec3 p) {
  vec3 q = (p - uBody.xyz) / uBody.w;
  float d = sdEllipsoid(q - vec3(0.0, 0.05, 0.0), vec3(0.42, 0.26, 0.42));
  float tilt0 = mix(1.45, 0.3, uBloom.x);
  for (int i = ZERO; i < 5; i++) {
    float a = uBloom.y + float(i) * 1.2566;
    float c = cos(a), s = sin(a);
    // into the petal's frame: x out along it, hinged at the heart's rim
    vec3 v = vec3(c * q.x + s * q.z, q.y, -s * q.x + c * q.z);
    v.x -= 0.18;
    float tilt = tilt0 + 0.04 * sin(uTime * 0.8 + float(i) * 1.7);
    float ct = cos(tilt), st = sin(tilt);
    v = vec3(ct * v.x + st * v.y, -st * v.x + ct * v.y, v.z);
    v.y -= 0.45 * v.z * v.z; // cupped
    d = smin(d, sdEllipsoid(v - vec3(0.62, 0.0, 0.0), vec3(0.66, 0.06, 0.36)), 0.08);
  }
  return d * uBody.w;
}

// The thread: a long capsule through the body's center, ringing in one harmonic.
float filamentSdf(vec3 p) {
  vec3 v = p - uBody.xyz;
  float h = max(uFil.w, 1e-4);
  float s = clamp(dot(v, uFil.xyz), -h, h);
  float wave = uFil2.z * sin(uFil2.w * 3.14159 * (s + h) / (2.0 * h)) + 0.012 * uFil2.y * sin(s * 2.3 - uTime * 2.0);
  return length(v - uFil.xyz * s - uFilSwing * wave) - uFil2.x;
}

// The box's own frame: turned to face its way, shivering when knocked.
vec3 boxLocal(vec3 p) {
  vec3 q = p - uBody.xyz;
  float c = gBoxTurn.x, s = gBoxTurn.y;
  q = vec3(c * q.x - s * q.z, q.y, s * q.x + c * q.z);
  q.x += gBoxShiver;
  return q;
}

// A box with the sphere's volume, its lid hinged along the back.
float boxSdf(vec3 p) {
  vec3 q = boxLocal(p);
  float s = HALF_SIDE * uBody.w;
  float lh = 0.12 * s;
  float k = 0.06 * s;
  float base = sdBox(q - vec3(0.0, -lh, 0.0), vec3(s, s - lh, s), k);
  // opened, it is hollow
  if (uBox.y > 0.0) base = max(base, -sdBox(q - vec3(0.0, 0.25 * s, 0.0), vec3(0.86 * s, s, 0.86 * s), 0.02 * s));
  // the lid, turned on its hinge (not at all while it is shut)
  float ca = gBoxTurn.z, sa = gBoxTurn.w;
  vec3 v = q - vec3(0.0, s - 2.0 * lh, -s);
  v = vec3(v.x, v.y * ca - v.z * sa, v.y * sa + v.z * ca);
  float lid = sdBox(v - vec3(0.0, lh, s), vec3(s, lh, s), k);
  return min(base, lid);
}

// The body, reshaped by whichever form it is taking. Mirrors Forms::shape in forms/mod.rs.
float bodySurface(vec3 p) {
  float d = cellsSurface(p);
  if (uBloom.x > 0.0) d = mix(d, flowerSdf(p), smoothstep(0.0, 0.35, uBloom.x));
  if (uFil2.y > 0.0) d = mix(d, filamentSdf(p), smoothstep(0.0, 0.15, uFil2.y));
  if (uBox.x > 0.0) d = mix(d, boxSdf(p), uBox.x);
  return d;
}

// How much of the floor at xz is liquid, 0..1.
float lakeMask(vec2 xz) { return smoothstep(uLake.z, uLake.z - 0.8, length(xz - uLake.xy)); }

// Height of the liquid floor: rings from the impact, rings from touches, a slow swell.
float lakeHeight(vec2 xz) {
  float dist = length(xz - uLake.xy);
  float m = smoothstep(uLake.z, uLake.z - 0.8, dist);
  if (m <= 0.0) return 0.0;
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
  vec4 tri3 = TRI[i];
  vec2 shape = TSHAPE[i];
  float d2 = sdTri2(q.xy, vec2(tri3.x, 0.0), vec2(tri3.y, 0.0), tri3.zw);
  float dz = abs(q.z) - shape.x * 0.5;
  // weather wears it down: the outline shrinks by as much as the edges round, so it keeps its size
  float w = uTriB[i].w;
  d2 += w; dz += w * 0.5;
  float tri = (min(max(d2, dz), 0.0) + length(max(vec2(d2, dz), 0.0)) - shape.y - w) * size;
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

// Where xz is along the river (x) and across it from its winding middle (y). Mirrors River::local.
vec2 riverLocal(vec2 xz) {
  vec2 d = xz - uRiver.xy;
  vec2 dir = gRiverDir;
  float s = dot(d, dir);
  return vec2(s, dot(d, vec2(-dir.y, dir.x)) - 2.2 * sin(0.21 * s) - 1.1 * sin(0.47 * s + 1.3));
}

// The column in row k (0..23) and lane l (-1..1): its slot, and how tall it stands.
int pillarSlot(float k, float l) { return int(k) * 3 + int(l) + 1; }
float pick4(vec4 v, int c) { return c == 0 ? v.x : c == 1 ? v.y : c == 2 ? v.z : v.w; }
float pillarHeight(int slot) { return pick4(uPil[slot >> 2], slot & 3); }

// The columns standing out of the river, and which one is nearest (16 + its slot).
// They stand on a lattice that follows the river's winding, so only the one in this
// cell is measured; any other is at least as far as the edge of its own cell.
float pillars(vec3 p, out int pid) {
  vec2 su = riverLocal(p.xz);
  float k = clamp(floor(su.x / PIL_GAP) + 12.0, 0.0, 23.0);
  float l = clamp(floor(su.y / PIL_GAP + 0.5), -1.0, 1.0);
  int slot = pillarSlot(k, l);
  float h = pillarHeight(slot);
  vec2 q = su - vec2((k - 11.5) * PIL_GAP, l * PIL_GAP);
  float d = 1e3;
  pid = 16 + slot;
  if (h > 0.02) d = sdBox(vec3(q.x, p.y - FLOOR_Y - h * 0.5, q.y), vec3(PIL_HALF, h * 0.5 + 0.02, PIL_HALF), 0.1);
  // any other is at least as far as the next row that has one standing, ahead or behind,
  // or across in this row
  int row = int(k);
  float rows = pick4(uPilRow[row >> 2], row & 3);
  float behind = floor(rows / 32.0);
  float ahead = rows - 32.0 * behind;
  d = min(d, min(ahead * PIL_GAP - q.x, behind * PIL_GAP + q.x) - PIL_HALF);
  for (int j = ZERO; j < 3; j++) {
    float lj = float(j - 1);
    if (lj != l && pillarHeight(pillarSlot(k, lj)) > 0.02) d = min(d, abs((lj - l) * PIL_GAP - q.y) - PIL_HALF);
  }
  // none stand further across than the outer lanes
  d = max(d, abs(su.y) - PIL_GAP - PIL_HALF);
  // the winding stretches distances; and there is nothing above the tallest, or under the ground
  return max(d * 0.6, max(p.y - FLOOR_Y - uRiver2.w - 0.1, FLOOR_Y - p.y));
}

// Also hands back the body alone, and the nearest piece and its distance, so
// shade() can use them without compiling bodySurface() and shards() again.
// One of the wound's pieces (wound.rs): a small core with four spikes, turning slowly, so
// that however it lies one points up. Not an exact distance, so it is taken at half.
float caltrop(vec3 q, float r, float seed) {
  float a = uTime * 0.4 + seed * 1.7;
  q.xz = mat2(cos(a), sin(a), -sin(a), cos(a)) * q.xz;
  vec3 n = q / max(length(q), 1e-5);
  float m = max(max(dot(n, vec3(0.577, 0.577, 0.577)), dot(n, vec3(-0.577, -0.577, 0.577))),
                max(dot(n, vec3(-0.577, 0.577, -0.577)), dot(n, vec3(0.577, -0.577, -0.577))));
  float m2 = m * m, m4 = m2 * m2, m8 = m4 * m4;
  return (length(q) - r * (0.6 + 1.0 * m8 * m8 * m2)) * 0.5;
}

float scene(vec3 p, out float body, out float piece, out int id) {
  // the fold repeats the body through mirrored space (forms/fold.rs)
  vec3 fsg, fk;
  float fang, fs;
  vec3 bp = foldSpace(p, fsg, fang, fk, fs);
  body = uBurst.x > 0.5 ? 1e3 : bodySurface(bp) * fs * (uFold.x > 0.0 ? foldStep(p) : 1.0);
  float d = body;
  piece = 1e3;
  id = -1;
  if (uBurst.w > 0.0) {
    // all the way into the river, the pieces are under the ground and need not be looked for
    if (uRiver2.x < 0.99) piece = shards(p, id);
    if (uRiver2.w > 0.0) {
      int pid;
      float pd = pillars(p, pid);
      if (pd < piece) { piece = pd; id = pid; }
    }
    d = min(d, piece);
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
    vec3 q = p - uDrop[i].xyz;
    float dd = length(q) - uDrop[i].w;
    if (((uCaltrop >> i) & 1) != 0) dd = caltrop(q, uDrop[i].w, float(i));
    d = smin(d, dd, uK.y);
  }
  // the other one, joined last and never smoothly: the two never melt into each other
  if (uOther.w > 0.0) {
    float od = otherSdf(p);
    if (od < piece) { piece = od; id = OTHER; }
    d = min(d, od);
  }
  return d;
}

// Where the ray passes through the sphere at c of radius r, widening span s to take it in;
// r below zero is no sphere. marchSpan() chooses with step() and pick() instead of branches.
// a where m is 0, b where m is 1, exactly. A select, not arithmetic: a * (1 - m) + b * m is
// free for a fast-math compiler to rewrite as a + (b - a) * m, which loses b entirely when a
// is 1e9. Safari on iPhone does exactly that, and the body's span collapsed to nothing.
float pick(float a, float b, float m) { return m > 0.5 ? b : a; }
vec2 pick(vec2 a, vec2 b, float m) { return m > 0.5 ? b : a; }
vec4 pick(vec4 a, vec4 b, float m) { return m > 0.5 ? b : a; }

vec2 spanSphere(vec3 ro, vec3 rd, vec3 c, float r, vec2 s) {
  vec3 oc = ro - c;
  float b = dot(oc, rd);
  float h = b * b - dot(oc, oc) + r * abs(r);
  float e = sqrt(max(h, 0.0));
  float m = step(0.0, h);
  return vec2(min(s.x, pick(1e9, -b - e, m)), max(s.y, pick(-1e9, -b + e, m)));
}

// 1.0 if a < b, else 0.0.
float below(float a, float b) { return 1.0 - step(b, a); }

// The stretches of the ray where scene() can reach zero: xy through the spheres that hold the
// body and everything joined to it, zw through the layer over the floor where the lake, the
// pieces and the columns are. Outside both there is nothing, so the march need not go there.
// Each bound is grown by how far the smooth joins can pull a surface out.
vec4 marchSpan(vec3 ro, vec3 rd) {
  float r = uBody.w;
  float lake = step(1e-9, uLake.z);
  float grow = 0.05 + 0.25 * (uK.x * float(uNH) + uK.y * float(uND)) + 0.12 * r * lake;
  vec2 s = vec2(1e9, -1e9);
  // the furthest the ripples, the gaze, the boiling, the buds and the furrow push the body out
  float disp = 0.012 + 0.31 * abs(uGaze.w) + 0.045 * abs(uAgit) + 0.32 * abs(uDiv.y) + 0.03 * abs(uScar.w) + 0.02 * abs(uWound.z);
  // the highest the lake's waves reach, and the highest piece
  float wave = 0.12;
  float top = -1e9;
  float pieces = step(1e-9, uBurst.w) * below(uRiver2.x, 0.99);
  for (int i = ZERO; i < 12; i++) {
    float fi = float(i);
    disp += below(fi, float(uNR)) * 0.26 * abs(uRip[min(i, 7)].w);
    disp += below(fi, float(uNB)) * 0.3 * abs(uBud[min(i, 4)].w);
    wave += below(fi, float(uNLR)) * 0.06 * abs(uLakeRip[min(i, 5)].w);
    float spikes = 1.0 + 0.6 * float((uCaltrop >> i) & 1);
    s = spanSphere(ro, rd, uDrop[i].xyz, pick(-1.0, uDrop[i].w * spikes + grow, below(fi, float(uND))), s);
    int h = min(i, 3);
    vec3 a = uHA[h].xyz, b = uHB[h].xyz;
    float rt = 0.5 * length(b - a) + max(max(uHA[h].w, uHB[h].w), uHBulb[h]) + 0.03 * r + grow;
    s = spanSphere(ro, rd, 0.5 * (a + b), pick(-1.0, rt, below(fi, float(uNH))), s);
    top = max(top, pick(-1e9, uTri[i].y + 0.874 * r * max(1.0, 0.5 * uTri[i].w) + 1.3 * uTri[i].w + 0.1 + grow, pieces));
  }
  float sc = max(max(uScale.x, uScale.y), max(uScale.z, 1.0));
  float cluster = step(0.5, float(uNC));
  float rb = (pick(2.0, max(2.5, 2.4 * uGrainR), cluster) + disp) * r * sc + grow * sc / max(min(uScale.x, uScale.y), 0.2);
  rb = max(rb, step(1e-9, uBloom.x) * (1.8 * r + grow));
  rb = max(rb, step(1e-9, uBox.x) * (3.0 * r + grow));
  rb = max(rb, step(1e-9, uFil2.y) * (uFil.w + uFil2.x + length(uFilSwing) * (abs(uFil2.z) + 0.02) + grow));
  // the slab it stands up into
  rb = max(rb, cluster * step(1e-9, uStone.y) * (length(uSlab.xyz - uBody.xyz) + 2.6 * r + grow));
  s = spanSphere(ro, rd, uBody.xyz, pick(-1.0, rb, step(uBurst.x, 0.5)), s);
  s = spanSphere(ro, rd, uOther.xyz, pick(-1.0, uOther.w * max(1.0, 1.6 + 0.6 * uOther2.z) * 1.02 + 0.02, step(1e-9, uOther.w)), s);
  // the lake, with the fountain's column and the drop it lets go of, and the columns
  float jet = step(-0.3, uJet.w) * (max(uJet.z, uJet.w + 0.3) + 0.3);
  top = max(top, pick(-1e9, FLOOR_Y + max(wave, jet) + grow / 0.8, lake));
  top = max(top, pick(-1e9, FLOOR_Y + uRiver2.w + 0.1 + grow / 0.6, step(1e-9, uBurst.w) * step(1e-9, uRiver2.w)));
  // the layer under that height: where the ray crosses it, and whether it comes down through it
  float down = step(rd.y, 0.0);
  float cross = (top - ro.y) / (abs(rd.y) < 1e-6 ? 1e-6 : rd.y);
  float there = step(-1e8, top) * max(down, step(0.0, cross));
  vec2 l = pick(vec2(1e9, -1e9), pick(vec2(0.0, cross), vec2(max(cross, 0.0), 1e9), down), there);
  // folded space repeats everywhere: march all of it
  return pick(vec4(s, l), vec4(0.0, 1e9, 1e9, -1e9), step(1e-9, uFold.x));
}

// Color at a surface point: blended through the same smooth joins as the shape.
// `d` is bodySurface(p), which the caller already has.
vec3 sceneColor(vec3 p, float d) {
  vec3 col = uBodyCol;
  // the seam a shade darker than the rest of it; the wound raw, then what it was given
  vec2 sc = scarAt(normalize(p - uBody.xyz + 1e-5));
  col = mix(col, vec3(0.36, 0.2, 0.22), 0.9 * sc.x * (1.0 - uWound.x));
  col = mix(col, mix(BLOOD, uWoundCol, uWound.y), clamp(sc.y * 1.4, 0.0, 1.0));
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

// The softbox overhead, normalize(-0.5, 0.8, 0.45), and the room's two softboxes.
const vec3 KEY = vec3(-0.478365, 0.765384, 0.430528);
const vec3 BOX_A = vec3(-0.496292, 0.744438, 0.446663);
const vec3 BOX_B = vec3(0.798007, 0.249377, -0.548630);

float pow4(float x) { x *= x; return x * x; }
float pow5(float x) { return pow4(x) * x; }

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
  float bd = dot(rd, vec3(0.282216, 0.940721, -0.188144)) * 3.5;
  float band = exp(-bd * bd);
  col += band * vec3(0.10, 0.08, 0.16) * (0.6 + 0.4 * hash(floor(rd * 400.0)));
  return col;
}

vec3 nightSky(vec3 rd) {
  return vec3(0.010, 0.012, 0.028) + stars(rd);
}

// Soft white studio: bright from above, gray toward the floor, two softboxes.
vec3 env(vec3 r) {
  vec3 c = mix(vec3(0.70, 0.71, 0.74), vec3(1.0), smoothstep(-0.35, 0.55, r.y));
  c += vec3(1.4) * smoothstep(0.90, 0.94, dot(r, BOX_A));
  c += vec3(0.75, 0.70, 0.66) * smoothstep(0.93, 0.965, dot(r, BOX_B));
  // a low strip of cold light behind it, and a warm one opposite: the only color the room has
  c += vec3(0.30, 0.42, 0.62) * smoothstep(0.12, 0.0, abs(r.y - 0.08)) * smoothstep(0.2, 0.8, -r.z);
  c += vec3(0.55, 0.36, 0.30) * smoothstep(0.1, 0.0, abs(r.y - 0.02)) * smoothstep(0.4, 0.9, r.x);
  c *= 1.0 - 0.3 * exp(-abs(r.y + 0.05) * 9.0);
  return c;
}

float sphOcc(vec3 pos, vec4 sph) {
  vec3 di = sph.xyz - pos;
  float l = length(di);
  return max(0.0, di.y / l) * (sph.w * sph.w) / (l * l);
}

// How much light reaches the matte white floor at p, under the soft shadows of everything
// above it. main() asks once per pixel, for whichever floor point it shows.
float floorShadow(vec3 p) {
  vec3 off = cellOffset();
  float rs = mix(uBody.w, cellRadius(), uCut.w) * 1.1;
  float occ = uCut.w < 1e-4 ? sphOcc(p, vec4(uBody.xyz, uBody.w * 1.1 * (1.0 - uFil2.y)))
                            : sphOcc(p, vec4(uBody.xyz - off, rs)) + sphOcc(p, vec4(uBody.xyz + off, rs));
  if (uFil2.y > 0.0) {
    // the thread's shadow: a line of soft beads along it
    for (int i = ZERO; i < 9; i++) {
      float s = (float(i) - 4.0) * 1.2 * uFil2.y;
      occ += sphOcc(p, vec4(uBody.xyz + uFil.xyz * s, uFil2.x * 1.6));
    }
  }
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
  if (uOther.w > 0.0) occ += sphOcc(p, vec4(uOther.xyz, uOther.w * 1.3));
  float fade = exp(-0.05 * dot(p.xz, p.xz));
  // the pieces cast their own shadows wherever they stand
  float socc = 0.0;
  if (uBurst.w > 0.0) {
    for (int i = ZERO; i < 12; i++) {
      float h = uTri[i].w * mix(0.12, 0.4, uTriB[i].y);
      socc += sphOcc(p, vec4(uTri[i].xyz + vec3(0.0, h, 0.0), uTri[i].w * mix(0.12, 0.38, uTriB[i].y)));
    }
  }
  return 1.0 - 0.55 * clamp(occ * fade + socc, 0.0, 1.0);
}

vec3 shadeLiquid(vec3 p, vec3 rd, vec3 n, float ao, float db, vec3 er);
vec3 galaxy(vec3 p, vec3 rd);
vec3 caustic(vec3 p);
vec3 drama(vec3 col, vec2 uv, vec2 c, vec3 w);

// Dark and wet, lit by the room; brighter on each heartbeat.
vec3 shadeBlood(vec3 rd, vec3 n, float depth) {
  vec3 L = KEY;
  float ndv = max(dot(n, -rd), 0.0);
  vec3 col = mix(vec3(0.62, 0.05, 0.09), BLOOD * 0.5, depth) * (0.55 + 0.45 * max(dot(n, L), 0.0));
  col *= 1.0 + 0.9 * uRiver2.z;
  vec3 r = reflect(rd, n);
  float fres = 0.03 + 0.97 * pow5(1.0 - ndv);
  col = mix(col, env(r) * vec3(1.0, 0.86, 0.86), fres * 0.85);
  return col + vec3(1.0, 0.9, 0.9) * pow(max(dot(r, L), 0.0), 160.0) * 0.9;
}

// The river on the floor at p: running, rippled where columns fell back in, dark in the
// middle and thin at the banks.
vec3 riverFloor(vec3 p, vec3 rd, vec3 bg) {
  vec2 su = riverLocal(p.xz);
  float w = (3.2 + 0.6 * sin(0.33 * su.x + 0.4)) * uRiver2.x;
  float m = smoothstep(w, w - 0.35, abs(su.y)) * smoothstep(uRiver.w, uRiver.w - 3.0, abs(su.x));
  if (m <= 0.0) return bg;
  // two trains of ripples running downstream, as a slope along and across it
  float f = uRiver2.y;
  float a1 = su.x * 1.7 + su.y * 0.6 - f * 2.6;
  float a2 = su.x * 3.1 - su.y * 1.3 - f * 4.0;
  vec2 g = 0.05 * cos(a1) * vec2(1.7, 0.6) + 0.025 * cos(a2) * vec2(3.1, -1.3);
  vec2 dir = vec2(cos(uRiver.z), sin(uRiver.z));
  // far off the ripples are too fine to see, only to shimmer
  g = (dir * g.x + vec2(-dir.y, dir.x) * g.y) * smoothstep(34.0, 14.0, length(p - uCamPos));
  for (int i = ZERO; i < 6; i++) {
    if (i >= uNSp) break;
    vec2 o = p.xz - uSplash[i].xy;
    float d = length(o) + 1e-3;
    float a = uSplash[i].z;
    g -= o / d * 0.9 * exp(-a * 1.4) * sin(d * 6.0 - a * 9.0) * smoothstep(a * 3.0 + 0.6, a * 3.0 - 0.4, d);
  }
  vec3 n = normalize(vec3(-g.x, 1.0, -g.y));
  vec3 col = shadeBlood(rd, n, smoothstep(0.3, 2.6, w - abs(su.y)));
  // the columns darken the river around their feet
  float k = clamp(floor(su.x / PIL_GAP) + 12.0, 0.0, 23.0);
  float l = clamp(floor(su.y / PIL_GAP + 0.5), -1.0, 1.0);
  vec2 q = abs(su - vec2((k - 11.5) * PIL_GAP, l * PIL_GAP)) - PIL_HALF;
  float h = pillarHeight(pillarSlot(k, l));
  col *= 1.0 - 0.6 * step(0.02, h) * exp(-max(max(q.x, q.y), 0.0) * 3.0);
  col = mix(col, bg, smoothstep(26.0, 48.0, length(p - uCamPos)));
  return mix(bg, col, m);
}

// A column out of the river: solid in one of the twelve's colors, or burning. Near its foot,
// and all over while it is still short, it is wet with the river.
vec3 shadePillar(vec3 p, vec3 rd, int slot, vec3 n, float ao, vec3 er) {
  vec3 L = KEY;
  float ndv = max(dot(n, -rd), 0.0);
  vec3 base = TCOL[slot % 12];
  float y = p.y - FLOOR_Y;
  vec3 col;
  if (fract(sin(float(slot) * 78.233) * 43758.547) > 0.6) {
    // energy: light climbing it, hottest at the edges, flaring on the beat
    float band = 0.5 + 0.5 * sin(y * 5.0 - uTime * 9.0 + float(slot));
    col = mix(base, vec3(1.0), 0.5) * (1.05 + 0.6 * band) + base * 1.4 * (1.0 - ndv) * (1.0 - ndv);
    col *= 1.0 + 0.6 * uRiver2.z;
  } else {
    col = base * (0.3 + 0.7 * max(dot(n, L), 0.0)) * (0.55 + 0.45 * ao);
    col += vec3(pow(max(dot(reflect(-L, n), -rd), 0.0), 40.0)) * 0.25;
    col = mix(col, er, 0.04 + 0.3 * pow5(1.0 - ndv));
  }
  float h = pillarHeight(slot);
  float wet = max(smoothstep(1.3, 0.05, y), 1.0 - smoothstep(0.6, 2.8, h));
  // and it runs down the sides in streaks
  wet = max(wet, smoothstep(0.55, 0.95, sin(dot(p.xz, vec2(7.0, 5.0)) + float(slot)) * 0.5 + 0.5) * smoothstep(h * 0.6, 0.0, y));
  // wet: dark red, and the light's highlight sharp on it
  vec3 blood = BLOOD * (0.6 + 0.6 * max(dot(n, L), 0.0)) * (1.0 + 0.9 * uRiver2.z)
             + vec3(1.0, 0.85, 0.85) * (pow(max(dot(reflect(-L, n), -rd), 0.0), 120.0) + 0.25 * pow4(1.0 - ndv));
  return mix(col, blood, wet);
}

// The liquid floor dissolves into the white with distance, so it has no far edge.
vec3 lakeFade(vec3 col, vec3 p, vec3 bg) {
  return mix(col, bg, smoothstep(6.0, 20.0, length(p - uCamPos)));
}

// `lake` is the liquid floor where this ray meets it, shaded by main(); `shadow` is floorShadow() there.
vec3 background(vec3 ro, vec3 rd, vec2 uv, vec3 lake, float shadow) {
  vec3 bg = vec3(0.972, 0.970, 0.962) - 0.03 * length(uv) * 0.5;
  if (rd.y < 0.0) {
    vec3 p = ro + rd * ((FLOOR_Y - ro.y) / rd.y);
    bg *= shadow;
    bg += caustic(p) * bg;
    if (uRiver.w > 0.0) bg = riverFloor(p, rd, bg);
    // past the marcher's reach the liquid floor still shows, flat
    if (uLake.z > 0.0) bg = mix(bg, lakeFade(lake, p, bg), lakeMask(p.xz));
  }
  if (uFold.x > 0.0) {
    // the white comes apart along the sky's own seams
    vec3 fsky = foldSky(rd);
    float grow = smoothstep(0.0, 0.25, uFold.x * 1.4 - (1.0 - min(dot(fsky, vec3(0.6)), 1.0)) * 0.9);
    bg = mix(bg, fsky, max(grow, smoothstep(0.6, 1.0, uFold.x)));
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

  vec3 L = KEY;
  col *= 0.9 + 0.1 * max(dot(n, L), 0.0);
  vec3 r = reflect(rd, n);
  col += vec3(pow(max(dot(r, L), 0.0), 90.0) * 0.8);
  col += vec3(pow(max(dot(r, BOX_B), 0.0), 40.0) * 0.12);
  return col;
}

// Polished black stone: almost nothing of its own, only what it reflects.
vec3 shadeStone(vec3 p, vec3 rd, vec3 n, vec3 er) {
  float ndv = clamp(dot(n, -rd), 0.0, 1.0);
  vec3 L = KEY;
  vec3 col = vec3(0.016, 0.016, 0.02) * (0.5 + 0.5 * max(dot(n, L), 0.0));
  vec3 r = reflect(rd, n);
  float fres = 0.05 + 0.95 * pow5(1.0 - ndv);
  vec3 room = er;
  if (uNight > 0.001) room = mix(room, nightSky(r) * 1.5, uNight);
  col += room * fres * 0.8;
  col += vec3(pow(max(dot(r, L), 0.0), 240.0)) * 0.6;
  return col;
}

// One of the twelve, in its own material.
vec3 shadeShard(vec3 p, vec3 rd, int i, vec3 n, vec3 er) {
  vec3 v = -rd;
  float ndv = max(dot(n, v), 0.0);
  vec3 L = KEY;
  vec3 base = TCOL[i];
  vec4 mat = TMAT[i];
  float wrap = mat.z;
  float diff = clamp((dot(n, L) + wrap) / (1.0 + wrap), 0.0, 1.0);
  vec3 col = base * (0.32 + 0.68 * diff);
  // light passing through the translucent ones, glowing at the thin edges
  col += base * mat.z * 0.35 * pow(1.0 - ndv, 1.5);
  if (mat.w > 0.0) col *= 1.0 - mat.w * hash(floor(p * 40.0));
  if (i == 6) col += base * 1.6 * (1.0 - ndv) * (1.0 - ndv) * (1.0 - ndv); // velvet sheen
  if (i == 0) {
    vec3 irid = 0.5 + 0.5 * cos(6.2831 * ((1.0 - ndv) * 1.4 + uTime * 0.02 + vec3(0.0, 0.33, 0.67)));
    col = mix(col, irid, 0.3 * (1.0 - ndv));
  }
  float fres = mat.y + (1.0 - mat.y) * pow5(1.0 - ndv) * 0.7;
  col = mix(col, er * (i == 1 ? base * 1.15 : vec3(1.0)), clamp(fres, 0.0, 1.0));
  col += vec3(pow(max(dot(reflect(-L, n), v), 0.0), mat.x)) * min(mat.x / 120.0, 1.0);
  return col;
}

// ---- the other one (other.rs): its material ----------------------------------
// Dark glass. The room is in its faces, each face a little different; light bent through
// an edge splits into colors, more of them the more times it has been given back.
vec3 shadeOther(vec3 p, vec3 rd, vec3 n, float ao, vec3 er) {
  vec3 v = -rd;
  float ndv = max(dot(n, v), 0.0);
  vec3 L = KEY;
  float face = hash(floor(n * 3.0 + 0.5));
  vec2 k = otherCuts(p);
  float edge = exp(-abs(k.x - k.y) * 26.0);
  // the colors part by how steeply the light crosses each face, and most at its edges
  vec3 fire = 0.5 + 0.5 * cos(6.2831 * (face * 0.7 + ndv * 1.6 + k.x * 3.0 + uTime * 0.03 + vec3(0.0, 0.33, 0.67)));
  float split = 0.12 + 0.5 * uOther3.a;
  vec3 col = vec3(0.02, 0.022, 0.04) * ao;
  col += uOther3.rgb * 0.35 * pow(1.0 - ndv, 1.5);
  col += env(refract(rd, n, 0.66)) * fire * split * (0.25 + 0.75 * (1.0 - ndv));
  // the room in its faces, each face held at its own angle
  float fres = 0.05 + 0.95 * pow5(1.0 - ndv);
  col = mix(col, er * vec3(0.8, 0.84, 0.96), clamp(0.12 + 0.5 * face * face + fres * 0.8, 0.0, 0.92));
  col += fire * edge * (0.25 + 0.9 * uOther3.a);
  col += vec3(pow(max(dot(reflect(-L, n), v), 0.0), 260.0)) * 1.5;
  // struck, it rings cold and bright
  col += vec3(0.74, 0.82, 1.0) * uOther2.w * (0.15 + 0.5 * (1.0 - ndv) * (1.0 - ndv) + edge);
  if (uNight > 0.001) {
    // in the dark only its edges show, burning
    vec3 night = vec3(0.01, 0.01, 0.025) * (0.5 + face) + fire * (edge * (0.6 + uOther3.a) + 0.08 * split);
    col = mix(col, night, uNight);
  }
  return col;
}
// ---- end of the other one's material ------------------------------------------

// `far`: the liquid floor past the marcher's reach, where only the plain liquid is wanted.
// main() probes the field around p in its march loop and hands over what it found:
// the normal, ambient occlusion, the body's distance, and the nearest piece.
vec3 shade(vec3 p, vec3 rd, vec2 uv, bool far, vec3 n, float ao, float db, float ds, int id, float shadow) {
  // the room in its skin, looked up once for whichever material it turns out to be
  vec3 er = env(reflect(rd, n));
  if (!far && id == OTHER && ds < db && ds < 0.01) return shadeOther(p, rd, n, ao, er);
  int shard = -1;
  if (uBurst.w > 0.0 && !far && id >= 0 && id != OTHER && ds < db) shard = id;
  if (shard >= 16) return shadePillar(p, rd, shard - 16, n, ao, er);
  // One call site for shadeLiquid, so the compiler builds it once here.
  vec3 col = far || shard >= 0 || uDiv.z < 0.999 ? shadeLiquid(p, rd, n, ao, db, er) : vec3(0.0);
  if (far) return col;
  if (shard >= 0) {
    // in flight it is still the liquid; it takes on its material as it lands
    return mix(col, shadeShard(p, rd, shard, n, er), uTriB[shard].y);
  }
  // a body that has been hurt flushes
  col *= mix(vec3(1.0), vec3(1.0, 0.72, 0.78), uBurst.z * 0.9);
  if (uLake.z > 0.0) {
    // where the floor is hit but hasn't turned liquid yet, it is still the white floor
    float floorness = smoothstep(0.3, 0.08, p.y - FLOOR_Y) * (1.0 - lakeMask(p.xz));
    vec3 bg = vec3(0.972, 0.970, 0.962) - 0.03 * length(uv) * 0.5;
    float lakeness = smoothstep(0.3, 0.08, p.y - FLOOR_Y) * lakeMask(p.xz);
    col = mix(col, lakeFade(col, p, bg), lakeness);
    col = mix(col, bg * shadow, floorness);
  }
  if (uDiv.z > 0.001 || uStone.x > 0.001) {
    if (uDiv.z > 0.001) col = mix(col, shadeCell(p, rd, n), uDiv.z);
    if (uStone.x > 0.001) col = mix(col, shadeStone(p, rd, n, er), uStone.x);
  }
  if (uOtherIn > 0.001) {
    // the other one, inside: a cold dark shape seen through the milk, its edges still splitting light
    vec3 r = refract(rd, n, 0.8);
    vec3 oc = uOther.xyz - p;
    float miss = length(oc - r * max(dot(oc, r), 0.0)) / (uOther.w * 1.5);
    float core = exp(-miss * miss * 2.2) * uOtherIn;
    vec3 fire = 0.5 + 0.5 * cos(6.2831 * (miss * 1.7 + uTime * 0.05 + vec3(0.0, 0.33, 0.67)));
    vec3 dark = vec3(0.03, 0.03, 0.06) + uOther3.rgb * 0.06 + fire * (0.1 + 0.3 * uOther3.a) * smoothstep(0.2, 0.9, miss);
    col = mix(col, dark, core * 0.85);
  }
  if (uCosmos.x > 0.001) {
    // every copy in the fold holds the same galaxy, seen from its own side
    vec3 fsg, fk;
    float fang, fs;
    vec3 gp = foldSpace(p, fsg, fang, fk, fs);
    vec3 grd = rd;
    grd.xz = foldRot(grd.xz, fang);
    grd *= fsg;
    vec3 gal = galaxy(uBody.xyz + (gp - uBody.xyz) * fs, grd);
    if (dot(fk, fk) > 0.0) gal *= mix(vec3(1.0), 0.6 + 0.9 * foldPal(hash(fk + 0.5) + uFold.y * 0.01), uFold.x * 0.8);
    col = mix(col, gal, uCosmos.x);
  }
  if (uBox.y > 0.0) {
    // inside the open box it is lit from within
    vec3 q = boxLocal(p);
    float s = HALF_SIDE * uBody.w;
    float inside = step(max(abs(q.x), abs(q.z)), 0.88 * s) * step(q.y, 0.75 * s);
    float deep = smoothstep(0.75 * s, -s, q.y);
    col = mix(col, vec3(1.0, 0.94, 0.78) * (0.9 + 0.9 * deep), inside * uBox.y * 0.85);
  }
  if (uNC == 0) {
    // the cut: a hot seam where the slice plane crosses the surface
    float seam = abs(dot(p - uBody.xyz, uCut.xyz)) / uBody.w;
    col += uDiv.x * vec3(1.0, 0.35, 0.65) * (exp(-seam * 60.0) * 1.4 + exp(-seam * 10.0) * 0.12);
  }
  return col;
}

// The lake as mirror, deep, and ice. `col` is the plain liquid underneath.
// `er` is env(reflect(rd, n)), which the caller already has.
vec3 mereSurface(vec3 p, vec3 rd, vec3 n, vec3 col, float fres, vec3 er) {
  vec3 r = reflect(rd, n);
  float ndv = max(dot(n, -rd), 0.0);
  if (uMere.x > 0.0) {
    // still: the room, perfectly, and the object standing in it where it fell
    vec3 mir = er * vec3(0.93, 0.94, 0.96);
    vec3 gc = vec3(uLake.x, FLOOR_Y + uBody.w, uLake.y);
    vec3 oc = p - gc;
    float b = dot(oc, r);
    float disc = b * b - dot(oc, oc) + uBody.w * uBody.w;
    if (disc > 0.0 && -b - sqrt(disc) > 0.0) {
      vec3 gn = normalize(p + r * (-b - sqrt(disc)) - gc);
      float gd = max(dot(gn, -r), 0.0);
      vec3 ghost = mix(vec3(0.90, 0.91, 0.95) * (0.8 + 0.2 * gn.y), env(reflect(r, gn)), 0.15 + 0.85 * pow4(1.0 - gd));
      mir = mix(mir, ghost, 0.92);
    }
    col = mix(col, mir, uMere.x);
  }
  if (uMere.y > 0.0) {
    // deep: ink, and the object a faint moon far under it
    float dd = length(p.xz - uLake.xy);
    vec3 ink = vec3(0.015, 0.02, 0.04) + er * fres * 0.7
             + vec3(0.85, 0.88, 1.0) * 0.4 * exp(-dd * dd * 0.6) * (0.8 + 0.2 * sin(uTime * 0.8));
    col = mix(col, ink, uMere.y);
  }
  float im = uMere.z * smoothstep(uIce.z, uIce.z - 0.6, length(p.xz - uIce.xy));
  if (im > 0.0) {
    // ice: frosted, pale, holding the shape it froze in, scored by every blow
    vec3 ice = mix(vec3(0.78, 0.86, 0.93), vec3(0.97, 0.99, 1.0), 0.5 + 0.5 * n.y);
    ice *= 0.93 + 0.07 * hash(floor(p * 55.0));
    ice = mix(ice, er, 0.2 + 0.6 * pow4(1.0 - ndv));
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

// A flower's colors: pollen at the heart, petals deep at the base and pale at the tips.
vec3 formTint(vec3 p, vec3 tint) {
  if (uBloom.x > 0.0) {
    vec3 q = (p - uBody.xyz) / uBody.w;
    float rho = length(q.xz);
    float heart = smoothstep(0.5, 0.36, rho) * step(-0.1, q.y);
    vec3 petal = mix(vec3(0.93, 0.42, 0.62), vec3(1.0, 0.95, 0.96), smoothstep(0.35, 1.45, rho));
    petal = mix(petal, tint, 0.3);
    vec3 pollen = vec3(0.98, 0.76, 0.26) * (0.85 + 0.15 * hash(floor(p * 60.0)));
    tint = mix(tint, mix(petal, pollen, heart), smoothstep(0.2, 0.7, uBloom.x));
  }
  return tint;
}

// Seen through its skin: a spiral galaxy turning in the middle, stars all the way down.
vec3 galaxy(vec3 p, vec3 rd) {
  vec3 c = uBody.xyz;
  float R = uBody.w;
  vec3 n = normalize(p - c);
  vec3 r = refract(rd, n, 0.8);
  vec3 col = vec3(0.012, 0.010, 0.032) + stars(r) * 0.9;
  vec3 ax = vec3(0.229658, 0.918630, 0.321521);
  float den = dot(r, ax);
  float t = dot(c - p, ax) / (abs(den) > 1e-3 ? den : 1e-3);
  if (t > 0.0) {
    vec3 q = p + r * t - c;
    vec3 u = normalize(cross(ax, vec3(0.0, 0.0, 1.0)));
    vec3 w = cross(ax, u);
    vec2 xy = vec2(dot(q, u), dot(q, w)) / R;
    float rho = length(xy);
    float phi = atan(xy.y, xy.x) - uCosmos.y;
    float arms = 0.5 + 0.5 * cos(2.0 * phi - uCosmos.w * 4.5 * log(rho + 0.04));
    arms *= arms * arms;
    float disc = exp(-rho * 3.0) * smoothstep(1.0, 0.6, rho);
    float dust = hash(floor(vec3(xy * 70.0, 3.0)));
    col += disc * arms * mix(vec3(0.45, 0.58, 1.0), vec3(1.0, 0.55, 0.78), step(0.9, dust)) * 1.7;
    col += disc * step(0.985, dust) * arms * 2.0;
    col += exp(-rho * rho * 55.0) * vec3(1.0, 0.86, 0.62) * (1.4 + 4.0 * uCosmos.z);
  }
  float ndv = max(dot(n, -rd), 0.0);
  float fres = 0.04 + 0.96 * pow5(1.0 - ndv);
  vec3 L = KEY;
  col = mix(col, env(reflect(rd, n)), clamp(fres * 0.9 + 0.04, 0.0, 1.0));
  col += vec3(pow(max(dot(reflect(-L, n), -rd), 0.0), 120.0));
  return col;
}

// ---- Color and drama ------------------------------------------------------
// Light, film and the big moments. Nothing here marches or calls scene(): it only
// colors what main() already found, so it adds no compile weight to the field.

// A soap film `d` thick (micrometres) seen at cosine `c`: the light that comes back,
// red, green and blue each cancelled at their own thickness.
vec3 filmColor(float d, float c) {
  float ct = sqrt(1.0 - (1.0 - c * c) / 1.77);
  return 0.5 - 0.5 * cos(4.0 * PI * 1.33 * d * ct / vec3(0.65, 0.53, 0.44));
}

// How thick the film is at surface direction n: thicker below, swirling slowly, thinned by gaze.
float filmThickness(vec3 n) {
  return 0.42 + 0.18 * n.y + 0.09 * sin(dot(n, vec3(3.1, 1.7, -2.3)) + uTime * 0.21)
         + 0.05 * sin(dot(n, vec3(-5.3, 4.1, 2.9)) - uTime * 0.13) - 0.12 * uGaze.w;
}

// The room seen through the body, each color bent its own amount.
vec3 dispersed(vec3 rd, vec3 n) {
  // one copy of the room, looked up once for each color
  vec3 c = vec3(0.0);
  for (int i = ZERO; i < 3; i++) {
    vec3 e = env(refract(rd, n, i == 0 ? 0.735 : i == 1 ? 0.75 : 0.77));
    c[i] = e[i];
  }
  return c;
}

// Light focused through the body onto the floor at p: a hot point inside its shadow
// with a ring of split color around it, wavering with the surface.
vec3 caustic(vec3 p) {
  if (uFine < 0.5 || uBurst.x > 0.5 || uNC > 0 || uBox.x > 0.5) return vec3(0.0);
  float h = max(uBody.y - FLOOR_Y, 0.0);
  vec2 o = (p.xz - uBody.xz) / (uBody.w * (0.8 + 0.25 * h));
  o += 0.06 * vec2(sin(o.y * 5.0 + uTime * 1.3), sin(o.x * 4.0 - uTime * 1.1));
  float r = length(o);
  vec3 rr = (r - vec3(0.66, 0.60, 0.54)) * 6.5;
  vec3 ring = exp(-rr * rr);
  float core = exp(-r * r * 7.0);
  vec3 light = normalize(uBodyCol + 0.05) * 1.73;
  float clear = (1.0 - uCoat.y) * (1.0 - uCosmos.x) * (1.0 - uNight) * exp(-h * 0.35);
  return (light * core * 0.45 + ring * light * 0.3) * clear * (1.0 + 0.8 * uAgit);
}

// Where the body is on screen, in the same units as uv.
vec2 bodyOnScreen() {
  vec3 rel = uBody.xyz - uCamPos;
  return vec2(dot(rel, uCamRight), dot(rel, uCamUp)) * uFocal / max(dot(rel, uCamFwd), 0.1);
}

// The color of the moment: what just happened decides it.
vec3 washColor(vec2 uv, vec2 c) {
  vec3 w = vec3(0.50, 0.66, 1.0);                                           // cold: a blow, a fall
  w = mix(w, vec3(1.0, 0.30, 0.62), clamp(uDiv.x * 3.0, 0.0, 1.0));          // the seam's pink
  w = mix(w, vec3(1.0, 0.82, 0.45), step(0.001, uBox.y));                     // the box's light
  w = mix(w, vec3(0.75, 0.04, 0.09), step(0.001, uRiver2.x));                 // the river's red
  float a = atan(uv.y - c.y, uv.x - c.x) / (2.0 * PI);
  vec3 spectrum = 0.55 + 0.45 * cos(2.0 * PI * (a + uTime * 0.05 + vec3(0.0, 0.33, 0.67)));
  return mix(w, spectrum, step(0.5, uBurst.x));                               // the twelve: every color at once
}

// Screen-wide: a ring of color thrown out from it on every shock, a kick of light,
// the edges closing in while it is upset, and a beat in them while it is wounded.
// `c` is bodyOnScreen() and `w` the washColor() of the moment, which main() shares with the pop.
vec3 drama(vec3 col, vec2 uv, vec2 c, vec3 w) {
  float s = clamp(uShock.x, 0.0, 1.0);
  float age = uShock.y;
  vec2 d = uv - c;
  float dist = length(d);
  if (s > 0.0) {
    // the ring: each color a little behind the last
    vec3 rad = age * 2.4 * vec3(1.0, 0.965, 0.93) + 0.1;
    vec3 rr = (dist - rad) * 9.0;
    vec3 ring = exp(-rr * rr) * exp(-age * 1.8) * s;
    col = mix(col, w * (0.55 + 0.45 * ring), clamp(ring * 0.55, 0.0, 1.0));
    // a flash of light, then the color of the moment over everything, fading
    col *= 1.0 + 0.45 * s * exp(-age * 16.0);
    col = mix(col, col * mix(vec3(1.0), w * 1.25, 0.6), 0.35 * s * exp(-age * 2.2));
  }
  // the room closes in around it
  float upset = max(uAgit, uBurst.z * (0.75 + 0.25 * sin(uTime * 7.5)));
  if (upset > 0.0 || s > 0.0) {
    float edge = smoothstep(0.5, 1.9, dist);
    vec3 bruise = mix(vec3(0.28, 0.22, 0.35), vec3(0.55, 0.08, 0.16), clamp(upset * 1.5, 0.0, 1.0));
    col = mix(col, col * bruise, edge * clamp(0.45 * upset + 0.4 * s, 0.0, 0.6) * (1.0 - 0.7 * uNight));
  }
  return col;
}

vec3 shadeLiquid(vec3 p, vec3 rd, vec3 n, float ao, float db, vec3 er) {
  vec3 tint = formTint(p, sceneColor(p, db));
  vec3 v = -rd;
  float ndv = max(dot(n, v), 0.0);
  vec3 L = KEY;

  // Milky body with light scattering through it.
  float wrap = clamp((dot(n, L) + 0.6) / 1.6, 0.0, 1.0);
  vec3 deep = vec3(0.70, 0.74, 0.84);
  vec3 lit = vec3(0.97, 0.97, 0.99);
  vec3 col = mix(deep * tint, lit * tint, wrap) * mix(0.8, 1.0, ao);
  // Upset, it darkens and flushes, and something hot shows through from inside:
  // brightest where it is thickest, beating while it is wounded.
  col = mix(col, col * vec3(0.75, 0.45, 0.5), uAgit * 0.6);
  float hurt = max(uAgit, uBurst.z * (0.8 + 0.2 * sin(uTime * 7.5)));
  col += vec3(1.0, 0.16, 0.30) * pow(ndv, 2.5) * hurt * 0.42;

  // The room seen through it, each color bent its own amount: faint, and only
  // where it looks straight through.
  if (uFine > 0.5) col = mix(col, dispersed(rd, n) * tint, 0.16 * ndv * ndv * (1.0 - uAgit));

  // A thin film over all of it: faint across its face, whole colors at the edges.
  float film = 1.0 - ndv;
  vec3 irid = filmColor(filmThickness(n), ndv);
  col = mix(col, irid * 1.05 + 0.08, (0.05 + 0.5 * film * film + 0.15 * uGaze.w) * (1.0 - 0.6 * uAgit));

  // Glossy reflection.
  float fres = 0.04 + 0.96 * pow5(1.0 - ndv);
  vec3 r = reflect(rd, n);
  col = mix(col, er * ao, clamp(fres * 0.9 + 0.08, 0.0, 1.0));
  float spec = pow(max(dot(reflect(-L, n), v), 0.0), 90.0);
  col += vec3(0.9) * spec;

  // The lake's own states (mere.rs), and what the object rose as.
  float lk = uLake.z > 0.0 ? smoothstep(0.3, 0.08, p.y - FLOOR_Y) * lakeMask(p.xz) : 0.0;
  if (lk > 0.0) col = mix(col, mereSurface(p, rd, n, col, fres, er), lk);
  if (uCoat.x > 0.0) col = mix(col, er * vec3(0.86, 0.88, 0.92) + vec3(spec), uCoat.x * (1.0 - lk));
  if (uCoat.y > 0.0) col = mix(col, vec3(0.018, 0.02, 0.032) + tint * 0.03 + er * fres * 0.8 + vec3(spec) * 0.6, uCoat.y * (1.0 - lk));

  if (uNight > 0.001) {
    // At night it becomes a lens: the sky seen through it, a rim of its own color.
    // The stars come back off its skin as well as through it, and its rim is the film's color.
    // one copy of the stars, looked through and then off the skin
    vec3 sky = vec3(0.0);
    for (int i = ZERO; i < 2; i++) sky += nightSky(i == 0 ? refract(rd, n, 0.75) : r) * (i == 0 ? 1.6 : 0.3 + 1.4 * fres);
    float rim = film * film * film;
    vec3 night = sky + tint * 0.05 + mix(tint, irid * 1.4, 0.5) * rim * 0.9
               + vec3(0.8, 0.85, 1.0) * spec * 0.4;
    col = mix(col, night, uNight);
  }
  return col;
}

void main() {
  vec2 uv = (2.0 * gl_FragCoord.xy - uRes) / uRes.y;
  vec3 ro = uCamPos;
  vec3 rd = normalize(uCamRight * uv.x + uCamUp * uv.y + uCamFwd * uFocal);

  // One loop evaluates scene() for the march and then, at the point it found,
  // for the normal (tetrahedral gradient) and ambient occlusion. That makes
  // this the only place scene() is compiled: every extra call site copies the
  // whole scene again, and Direct3D's shader compiler (Edge and Chrome on
  // Windows) can take long enough on a big shader to crash the browser.
  sceneSetup();
  float t = 0.0;
  bool hit = false;
  bool marching = true;
  bool far = false;
  vec3 sp = ro;
  vec3 g = vec3(0.0);
  vec3 n = vec3(0.0, 1.0, 0.0);
  float ao = 0.0;
  // the four gradient samples sit symmetrically around p, so their mean is the value at p
  float db = 0.0;
  float ds = 0.0;
  int id = -1;
  int j = 0;
  // the landscape is far bigger than the room, so look further when it exists
  float tMax = 14.0 + 36.0 * uBurst.w + 18.0 * uFold.x;
  // march only where something can be: start where the ray first meets it, leap the gap
  // between the spheres and the floor's layer, and stop once past both
  vec4 span = marchSpan(ro, rd);
  t = max(min(span.x, span.z), 0.0);
  float tEnd = min(tMax, max(span.y, span.w));
  float om = 1.3, tPrev = t, prevR = 0.0;
  for (int i = ZERO; i < 148; i++) {
    if (marching && (hit || t > tEnd || i >= 140)) {
      marching = false;
      // past the marcher's reach the liquid floor is shaded where the ray meets the floor
      sp = hit ? ro + rd * t : ro + rd * ((FLOOR_Y - ro.y) / min(rd.y, -1e-4));
      far = !hit && rd.y < 0.0 && uLake.z > 0.0 && lakeMask(sp.xz) > 0.0;
      if (!hit && !far) break;
    }
    vec3 k = 2.0 * vec3((((j + 3) >> 1) & 1), ((j >> 1) & 1), (j & 1)) - 1.0;
    float h = 0.06 * float(j - 3);
    if (j == 4) n = normalize(g);
    vec3 q = marching ? ro + rd * t : (j < 4 ? sp + k * 0.0015 : sp + n * h);
    float body, piece;
    int near;
    float d = scene(q, body, piece, near);
    if (marching) {
      float rr = d * 0.75;
      if (om > 1.0 && rr + prevR < t - tPrev) { t = tPrev + prevR; om = 1.0; }
      else if (d < 0.0012 * t) hit = true;
      else {
        tPrev = t;
        prevR = rr;
        t += rr * om;
        if (t > span.y && t < span.z) { t = span.z; tPrev = t; prevR = 0.0; }
        if (t > span.w && t < span.x) { t = span.x; tPrev = t; prevR = 0.0; }
      }
    } else {
      if (j < 4) {
        g += k * d;
        db += body * 0.25;
        ds += piece * 0.25;
        id = near;
      } else {
        ao += (h - d) / h;
      }
      j++;
      if (j == 8) break;
    }
  }
  ao = clamp(1.0 - 0.18 * ao, 0.0, 1.0);

  // in the fold, the far copies go into the sky
  float fog = hit ? uFold.x * smoothstep(6.0, tMax, t) : 1.0;
  // the floor's shadows, measured once: where it touches the liquid floor, or where the ray meets the floor
  float shadow = 1.0;
  bool onFloor = hit && uLake.z > 0.0 && sp.y - FLOOR_Y < 0.3;
  if (onFloor || (fog > 0.0 && rd.y < 0.0)) shadow = floorShadow(onFloor ? sp : ro + rd * ((FLOOR_Y - ro.y) / rd.y));
  vec3 surf = hit || far ? shade(sp, rd, uv, far, n, ao, db, ds, id, shadow) : vec3(0.0);
  vec3 col = fog > 0.0 ? mix(surf, background(ro, rd, uv, surf, shadow), fog) : surf;
  if (uBox.y > 0.0) {
    // light rising out of the open box
    vec2 dxz = rd.xz;
    float s = max(dot(uBody.xz - ro.xz, dxz) / max(dot(dxz, dxz), 1e-5), 0.0);
    if (!hit || s < t) {
      vec3 at = ro + rd * s;
      float top = uBody.y + HALF_SIDE * uBody.w * 0.8;
      float off = length(at.xz - uBody.xz) / uBody.w;
      float above = smoothstep(top, top + 0.3, at.y) * exp(-(at.y - top) * 0.35);
      col += vec3(1.0, 0.93, 0.75) * uBox.y * above * exp(-off * off * 3.0) * 0.5;
    }
  }
  vec2 bc = bodyOnScreen();
  vec3 wash = uShock.x > 0.0 || uBurst.y > 0.0 ? washColor(uv, bc) : vec3(0.0);
  col = drama(col, uv, bc, wash);
  // the pop goes white, and comes back through every color
  if (uBurst.y > 0.0) {
    col = mix(col, vec3(1.0), uBurst.y);
    col = mix(col, wash, uBurst.y * (1.0 - uBurst.y) * 1.4);
  }
  col = pow(col, vec3(0.97));
  outColor = vec4(col, 1.0);
}
