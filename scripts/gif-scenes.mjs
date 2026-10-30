// What scripts/gif.mjs films. Each scene sets up the page (`saved`: localStorage before it
// opens, `NOW` replaced by the time in seconds), plays its fingers on `t`, and calls t.roll()
// where the GIF starts; t.cut() stops filming. Places are measured from the middle of the view
// in view heights (x right, y down), so a scene films the same at any size.
//
// Working one out: put `await t.snap('label')` where you want to look, run it, and open
// <out>/<scene>-label.png. Waits before t.roll() cost almost nothing, so get there the long way.

const TAU = Math.PI * 2;

// Five quick pinches around it, then long enough for it to open.
async function flower(t) {
  for (let i = 0; i < 5; i++) {
    const a = (i * TAU) / 5;
    const [dx, dy] = [Math.cos(a), Math.sin(a)];
    const [x0, y0] = [dx * 0.083, dy * 0.083];
    await t.drag([[x0, y0], [x0 + dx * 0.07, y0 + dy * 0.07]], 0.06);
    await t.wait(0.6);
  }
  await t.wait(4);
}

// Round and round the flower, `turns` times (counterclockwise on screen if positive), over `secs`.
async function spin(t, turns, secs, r = 0.16) {
  const n = Math.round(Math.abs(turns) * 48);
  const path = [];
  for (let k = 0; k <= n; k++) {
    const a = (k / n) * turns * TAU;
    path.push([r * Math.cos(a), -r * Math.sin(a)]);
  }
  await t.drag(path, secs);
}

// Where the middle of the floor is once it has gone to dust and the camera looks down.
const FLOOR = [0, 0.13];

// Strokes from all round the floor into its middle.
async function sweep(t, strokes, secs) {
  for (let i = 0; i < strokes; i++) {
    const a = (i * TAU) / strokes + 0.2;
    await t.drag([[FLOOR[0] + Math.cos(a) * 0.6, FLOOR[1] + Math.sin(a) * 0.3], FLOOR], secs);
    await t.wait(0.05);
  }
}

// A slice from above the view to below it, through the middle.
const cut = (t, x = 0) => t.drag([[x, -0.47], [x, 0.48]], 0.1);

// Two fingers that begin at `a` and `b` and close to `a2` and `b2` over `secs`, then lift.
async function twoFingers(t, a, b, a2, b2, secs) {
  await t.down(1, ...a);
  await t.down(2, ...b);
  const n = Math.max(1, Math.round(secs / 0.02));
  for (let k = 1; k <= n; k++) {
    const f = k / n;
    await t.move(1, a[0] + (a2[0] - a[0]) * f, a[1] + (a2[1] - a[1]) * f);
    await t.move(2, b[0] + (b2[0] - b[0]) * f, b[1] + (b2[1] - b[1]) * f);
    await t.wait(0.02);
  }
  await t.up(1, ...a2);
  await t.up(2, ...b2);
}

export default {
  sway: {
    about: 'the flower spun, not quite enough: it sways on itself and rights itself',
    async play(t) {
      await flower(t);
      t.roll();
      await spin(t, 3, 3.2);
      await t.wait(3.5);
    },
  },
  ropes: {
    about: 'the flower spun left past dizzy winds into ropes that fly out, lie down and are taken back in',
    async play(t) {
      await flower(t);
      await spin(t, 2, 2.2);
      t.roll();
      await spin(t, 3, 2.6);
      await t.wait(10);
    },
  },
  aloft: {
    about: 'the flower spun right past dizzy lays flat, whirls up off the floor and comes down again',
    async play(t) {
      await flower(t);
      await spin(t, -2, 2.2);
      t.roll(1.5);
      await spin(t, -3, 2.6);
      await t.wait(13);
    },
  },
  trackpad: {
    about: 'two fingers pinched on a trackpad squeeze it as far as two fingers on a screen; spread, it stands taller',
    async play(t) {
      await t.wait(1);
      t.roll();
      await t.wait(0.4);
      const { mouse, keyboard } = t.page;
      const { width: w, height: h } = t.page.viewportSize();
      await mouse.move(w / 2, h / 2);
      await keyboard.down('Control');
      for (let i = 0; i < 24; i++) {
        await mouse.wheel(0, 4);
        await t.wait(0.02);
      }
      await t.wait(0.3);
      for (let i = 0; i < 40; i++) {
        await mouse.wheel(0, -4);
        await t.wait(0.02);
      }
      await keyboard.up('Control');
      await t.wait(1.5);
    },
  },
  fingers: {
    about: 'on a phone, two fingers pinched together on it and let go leave a bud between them',
    size: [320, 520],
    async play(t) {
      await t.wait(1);
      t.roll();
      await t.wait(0.4);
      await twoFingers(t, [0.02, -0.09], [0.14, -0.09], [0.07, -0.09], [0.09, -0.09], 0.16);
      await t.wait(0.9);
      await twoFingers(t, [-0.15, 0.03], [-0.07, 0.11], [-0.12, 0.06], [-0.1, 0.08], 0.16);
      await t.wait(0.6);
      await t.snap('buds');
      await t.wait(1.2);
    },
  },
  almost: {
    about: 'a pinch let go of too late nearly leaves a bud, which sinks back; let go at once, it stays',
    async play(t) {
      await t.wait(1);
      t.roll();
      await t.wait(0.4);
      await t.drag([[0.12, -0.04], [0.19, -0.065]], 0.15, 1, { lift: false });
      await t.wait(0.45);
      await t.up(1, 0.19, -0.065);
      await t.wait(0.35);
      await t.snap('nearly');
      await t.wait(1.6);
      await t.drag([[0.12, -0.04], [0.19, -0.065]], 0.06);
      await t.wait(0.5);
      await t.snap('bud');
      await t.wait(1.2);
    },
  },
  cells: {
    about: 'cut, it is afraid; left alone it calms and burns; cut again, afraid again',
    async play(t) {
      await t.wait(1);
      t.roll();
      await t.wait(0.4);
      await cut(t);
      await t.wait(2.4);
      t.roll(8);
      await t.wait(24);
      t.roll();
      await t.wait(1);
      await cut(t, 0.28);
      await t.wait(2.5);
    },
  },
  flinch: {
    about: 'the flower, pinched a little too slowly, only flinches; pinched at once, it comes apart',
    async play(t) {
      await t.wait(1);
      await flower(t);
      t.roll();
      await t.wait(0.5);
      await t.drag([[0, 0], [0, 0.042]], 0.06, 1, { lift: false });
      await t.wait(0.5);
      await t.up(1, 0, 0.042);
      await t.wait(0.3);
      await t.snap('flinch');
      await t.wait(1.6);
      await t.drag([[0, 0], [0, 0.042]], 0.04);
      await t.wait(2.5);
    },
  },
  cut: {
    about: 'a cut runs through the whole room for a moment; cut again, the light runs the new way',
    async play(t) {
      await t.wait(1);
      t.roll();
      await t.wait(0.3);
      await t.drag([[0.12, -0.47], [-0.06, 0.48]], 0.1);
      await t.wait(4);
      await t.drag([[-0.5, -0.1], [0.5, 0.05]], 0.1);
      await t.wait(1.8);
    },
  },
  burst: {
    about: 'poked until it cannot hold any more: its heat on the floor, then every color at once, thrown out in rays',
    async play(t) {
      await t.wait(1);
      t.roll();
      for (let i = 0; i < 12; i++) {
        await t.tap(0, 0);
        await t.wait(0.2);
      }
      await t.wait(2.5);
    },
  },
  night: {
    about: 'drops torn free all round it, the room goes dark, and the night rings it',
    async play(t) {
      await t.wait(1);
      t.roll(3);
      for (let i = 0; i < 8; i++) {
        const a = (i * TAU) / 8;
        await t.drag([[Math.cos(a) * 0.1, Math.sin(a) * 0.1], [Math.cos(a) * 0.6, Math.sin(a) * 0.6]], 1);
        await t.wait(0.3);
      }
      t.roll();
      await t.wait(4);
    },
  },
  dust: {
    about: 'the flower, pinched, comes apart and settles as dust',
    async play(t) {
      await t.wait(1);
      await flower(t);
      t.roll();
      await t.wait(0.6);
      await t.drag([[0, 0], [0, 0.042]], 0.04);
      await t.wait(4.5);
    },
  },
  mound: {
    about: 'the dust swept together heaps itself into a mound',
    async play(t) {
      await t.wait(1);
      await flower(t);
      await t.drag([[0, 0], [0, 0.042]], 0.04);
      await t.wait(4);
      t.roll(2);
      await sweep(t, 16, 0.25);
      t.roll();
      await t.wait(4);
    },
  },
  tree: {
    about: 'tended, what roots in the mound grows into a tree and blossoms',
    async play(t) {
      await t.wait(1);
      await flower(t);
      await t.drag([[0, 0], [0, 0.042]], 0.04);
      await t.wait(4);
      await sweep(t, 16, 0.25);
      await t.wait(8);
      t.roll(3);
      for (let i = 0; i < 12; i++) {
        await t.tap(FLOOR[0], FLOOR[1] - 0.02);
        await t.wait(1.2);
      }
      await t.wait(4);
    },
  },
  scar: {
    about: 'scarred, hurt again, it opens and lets the pieces out',
    saved: { 'the-object.remembrance': 'v=1;f=1;born=NOW;seen=NOW;at=NOW;hurt=3,0;scar=0.35,0.25,0.9,0.6' },
    async play(t) {
      await t.wait(2);
      t.roll();
      await t.wait(0.8);
      for (let i = 0; i < 7; i++) {
        await t.tap(0, 0);
        await t.wait(0.06);
      }
      await t.wait(4.5);
    },
  },
  greeting: {
    about: 'greeted by someone it remembers kindly, it turns and answers in its own signs',
    voice: true,
    saved: {
      'the-object.found': 'poke,stretch,tear,merge,squeeze,fling,look around,warm,mix,provoke,patience,open,close,return,call,divide',
      'the-object.remembrance': 'v=1;f=1;born=NOW;seen=NOW;at=NOW;held=15,15;sang=6,6',
    },
    async play(t) {
      await t.asked('speak', 90);
      await t.wait(1.5);
      await t.click('toast');
      await t.asked('say something', 120);
      await t.wait(1.5);
      await t.click('toast');
      await t.wait(4);
      t.roll();
      await t.wait(1);
      await t.say('hello');
      await t.wait(5);
    },
  },
  mic: {
    about: 'from the first moment, the microphone waits in the corner; tapped and allowed, it fades and the mark for hearing comes',
    voice: true,
    async play(t) {
      await t.wait(2);
      t.roll();
      await t.wait(2.5);
      await t.click('mic');
      await t.wait(3.5);
    },
  },
  gather: {
    about: 'poked to pieces; held still on the ground, the pieces come home and it grows back inside them',
    async play(t) {
      await t.wait(1);
      for (let i = 0; i < 12; i++) {
        await t.tap(0, 0);
        await t.wait(0.2);
      }
      await t.wait(8);
      t.roll();
      await t.hold(0, 0.47, 1.6);
      await t.wait(3.5);
    },
  },
  box: {
    about: 'a square drawn around it: it takes the corners, and the light under it goes as it does',
    async play(t) {
      await t.wait(1.5);
      t.roll();
      await t.drag([[-0.417, -0.367], [0.417, -0.367], [0.417, 0.367], [-0.417, 0.367], [-0.417, -0.367]], 1.7);
      await t.wait(3);
    },
  },
};
