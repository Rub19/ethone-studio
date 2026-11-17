// Ethone Studio site: 3D hero (three.js), spring-driven motion, Ctrl+K demo, real downloads.
// Springs follow Apple's damping/response model: they start from the current value and can be
// redirected at any moment, so nothing ever jumps or locks out the pointer.

const RELEASE = 'https://github.com/Rub19/ethone-studio/releases/latest/download/';
const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

const APPS = [
  { id: 'pixel', name: 'Pixel', accent: '255 59 107', tagline: 'Retouche photo et compositing par calques.', tags: ['Calques et masques', '38 outils', 'Fichiers PSD'], size: '23 Mo' },
  { id: 'lumen', name: 'Lumen', accent: '30 200 140', tagline: 'Développement RAW et catalogue photo.', tags: ['RAW', 'Looks pellicule', 'Préréglages Lightroom'], size: '23 Mo' },
  { id: 'vector', name: 'Vector', accent: '255 149 10', tagline: 'Dessin vectoriel, logos et typographie.', tags: ['2 359 icônes', 'Dégradés', 'SVG optimisé'], size: '23 Mo' },
  { id: 'cut', name: 'Cut', accent: '168 85 247', tagline: 'Montage vidéo multipiste.', tags: ['Multipiste', 'Transitions', 'Médias Premiere'], size: '22 Mo' },
  { id: 'motion', name: 'Motion', accent: '30 170 240', tagline: 'Animation, titres et effets visuels.', tags: ['Images clés', 'Courbes', 'Formats 9:16'], size: '23 Mo' },
  { id: 'page', name: 'Page', accent: '250 100 50', tagline: 'Mise en page, magazines et PDF.', tags: ['Gabarits', 'InDesign IDML', 'Export PDF'], size: '24 Mo' },
  { id: 'folio', name: 'Folio', accent: '235 50 60', tagline: 'Lire, annoter, signer et organiser tes PDF.', tags: ['OCR', 'Signature', 'Organisation'], size: '25 Mo' },
  { id: 'sound', name: 'Sound', accent: '100 95 240', tagline: 'Édition et nettoyage audio.', tags: ['311 bruitages', 'Spectrogramme', 'Effets voix'], size: '24 Mo' },
];

const FORMATS = [
  { ext: 'PSD', app: 'Pixel', what: 'calque par calque', c: '255 59 107' },
  { ext: 'AI', app: 'Folio', what: 'compatible PDF', c: '235 50 60' },
  { ext: 'XMP', app: 'Lumen', what: 'préréglages Lightroom', c: '30 200 140' },
  { ext: 'CUBE', app: 'Lumen', what: 'LUT 3D', c: '30 200 140' },
  { ext: 'ASE', app: 'Vector', what: 'nuanciers Adobe', c: '255 149 10' },
  { ext: 'ACO', app: 'Pixel', what: 'nuanciers Photoshop', c: '255 59 107' },
  { ext: 'IDML', app: 'Page', what: 'documents InDesign', c: '250 100 50' },
  { ext: 'PRPROJ', app: 'Cut', what: 'médias Premiere', c: '168 85 247' },
];

const DEMO = [
  { q: 'filigrane', items: [['Filigrane « CONFIDENTIEL » rouge en diagonale au centre · 30 %', 'Folio'], ['Filigrane « PAYÉ » vert en bas à droite · 50 %', 'Folio'], ['Retirer le filigrane de la sélection (sans IA)', 'Pixel']] },
  { q: 'vinyle', items: [['Outil : Disque vinyle', 'Sound'], ['Outil : Cassette usée', 'Sound'], ['Outil : Lo-fi 8 kHz', 'Sound']] },
  { q: 'néon', items: [['Outil : Effet néon', 'Vector'], ['Pixel art 8 px · néon', 'Pixel'], ['Palette corail 0° · triade · néon', 'Vector']] },
  { q: 'préférences', items: [['Préférences…', 'Ctrl+,'], ['Interface : couleur d\'accent Violet', 'Toutes les apps'], ['Interface : taille 110 %', 'Toutes les apps']] },
];

/* ---------- Springs ---------- */

// Critically damped by default (damping 1.0); response is the time to settle, in seconds.
class Spring {
  constructor(value = 0, { damping = 1, response = 0.4 } = {}) {
    this.value = value;
    this.target = value;
    this.velocity = 0;
    this.k = (2 * Math.PI / response) ** 2;
    this.c = 4 * Math.PI * damping / response;
  }
  step(dt) {
    const a = -this.k * (this.value - this.target) - this.c * this.velocity;
    this.velocity += a * dt;
    this.value += this.velocity * dt;
    return this.value;
  }
  get settled() {
    return Math.abs(this.value - this.target) < 1e-4 && Math.abs(this.velocity) < 1e-3;
  }
}

/* ---------- Page content ---------- */

function buildCards() {
  const grid = document.getElementById('app-grid');
  for (const a of APPS) {
    const card = document.createElement('a');
    card.className = 'card reveal';
    card.href = `#dl-${a.id}`;
    card.style.setProperty('--accent', a.accent);
    card.innerHTML = `
      <img src="/icons/${a.id}.png" alt="" width="64" height="64" loading="lazy">
      <div><h3>${a.name}</h3><p>${a.tagline}</p></div>
      <ul>${a.tags.map((t) => `<li>${t}</li>`).join('')}</ul>
      <span class="more">Installer ${a.name} →</span>`;
    grid.append(card);
    if (!reduceMotion) tilt(card);
  }
}

// The card tilts toward the pointer and settles back with the same spring when it leaves.
function tilt(card) {
  const rx = new Spring(0, { response: 0.45 });
  const ry = new Spring(0, { response: 0.45 });
  const lift = new Spring(0, { response: 0.35 });
  let running = false;
  let last = 0;
  const frame = (t) => {
    const dt = Math.min((t - last) / 1000, 1 / 30);
    last = t;
    rx.step(dt); ry.step(dt); lift.step(dt);
    card.style.transform = `rotateX(${rx.value}deg) rotateY(${ry.value}deg) translateZ(${lift.value}px)`;
    if (rx.settled && ry.settled && lift.settled) {
      running = false;
      return;
    }
    requestAnimationFrame(frame);
  };
  const kick = () => {
    if (!running) {
      running = true;
      last = performance.now();
      requestAnimationFrame(frame);
    }
  };
  card.addEventListener('pointermove', (e) => {
    if (e.pointerType !== 'mouse') return;
    const r = card.getBoundingClientRect();
    const u = (e.clientX - r.left) / r.width;
    const v = (e.clientY - r.top) / r.height;
    ry.target = (u - 0.5) * 12;
    rx.target = (0.5 - v) * 10;
    lift.target = 18;
    card.style.setProperty('--mx', `${u * 100}%`);
    card.style.setProperty('--my', `${v * 100}%`);
    card.style.setProperty('--glow', '1');
    kick();
  });
  card.addEventListener('pointerleave', () => {
    rx.target = 0; ry.target = 0; lift.target = 0;
    card.style.setProperty('--glow', '0');
    kick();
  });
}

function buildFormats() {
  const ul = document.getElementById('chips');
  for (const f of FORMATS) {
    const li = document.createElement('li');
    li.style.setProperty('--c', f.c);
    li.innerHTML = `<b>.${f.ext.toLowerCase()}</b><span><strong>${f.app}</strong> · ${f.what}</span>`;
    ul.append(li);
  }
}

function buildDownloads() {
  const list = document.getElementById('dl-list');
  for (const a of APPS) {
    const row = document.createElement('div');
    row.className = 'dl-item';
    row.id = `dl-${a.id}`;
    row.innerHTML = `
      <img src="/icons/${a.id}.png" alt="" width="40" height="40" loading="lazy">
      <div><strong>${a.name}</strong><span>${a.size} · Windows 10 et 11</span></div>
      <div class="links">
        <a href="${RELEASE}${a.name}-Setup.exe" aria-label="Télécharger ${a.name} (installateur complet)">Installer</a>
        <a href="${RELEASE}${a.name}-Web-Setup.exe" aria-label="Télécharger ${a.name} (installateur web)">Web</a>
      </div>`;
    list.append(row);
  }
}

/* ---------- Scroll: reveals, counters, nav material ---------- */

function reveals() {
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      e.target.classList.add('in');
      io.unobserve(e.target);
    }
  }, { rootMargin: '0px 0px -8% 0px', threshold: 0.08 });
  document.querySelectorAll('.reveal').forEach((el, i) => {
    el.style.transitionDelay = el.classList.contains('card') ? `${(i % 4) * 60}ms` : '0ms';
    io.observe(el);
  });
}

function counters() {
  const fmt = (n) => n.toLocaleString('fr-FR');
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      io.unobserve(e.target);
      const el = e.target;
      const end = Number(el.dataset.count);
      const suffix = (el.dataset.plus !== undefined ? '+' : '') + (el.dataset.suffix || '');
      if (reduceMotion || end === 0) {
        el.textContent = fmt(end) + suffix;
        continue;
      }
      const t0 = performance.now();
      const dur = 1400;
      const tick = (t) => {
        const p = Math.min((t - t0) / dur, 1);
        const eased = 1 - (1 - p) ** 4;
        el.textContent = fmt(Math.round(end * eased)) + (p === 1 ? suffix : '');
        if (p < 1) requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    }
  }, { threshold: 0.6 });
  document.querySelectorAll('[data-count]').forEach((el) => io.observe(el));
}

function navMaterial() {
  const nav = document.getElementById('nav');
  const onScroll = () => nav.classList.toggle('scrolled', scrollY > 8);
  addEventListener('scroll', onScroll, { passive: true });
  onScroll();
}

/* ---------- Ctrl+K demo ---------- */

function paletteDemo() {
  const typed = document.getElementById('typed');
  const results = document.getElementById('results');
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  let visible = false;
  new IntersectionObserver(([e]) => { visible = e.isIntersecting; }).observe(results);
  const show = (items) => {
    results.innerHTML = '';
    items.forEach(([label, where], i) => {
      const li = document.createElement('li');
      li.innerHTML = `<span>${label}</span><small>${where}</small>`;
      if (i === 0) li.classList.add('on');
      results.append(li);
      setTimeout(() => li.classList.add('in'), reduceMotion ? 0 : 70 * i);
    });
  };
  if (reduceMotion) {
    typed.textContent = DEMO[0].q;
    show(DEMO[0].items);
    return;
  }
  (async () => {
    for (let k = 0; ; k = (k + 1) % DEMO.length) {
      while (!visible) await wait(400);
      const { q, items } = DEMO[k];
      results.innerHTML = '';
      for (let i = 1; i <= q.length; i++) {
        typed.textContent = q.slice(0, i);
        await wait(70 + Math.random() * 60);
      }
      show(items);
      await wait(2600);
      for (let i = q.length; i >= 0; i--) {
        typed.textContent = q.slice(0, i);
        await wait(28);
      }
      await wait(300);
    }
  })();
}

/* ---------- 3D hero ---------- */

async function hero() {
  const canvas = document.getElementById('scene');
  const fallback = document.getElementById('fallback');
  const label = document.getElementById('tile-label');
  const ids = ['studio', ...APPS.map((a) => a.id)];
  fallback.innerHTML = ids.map((id) => `<img src="/icons/${id}.png" alt="">`).join('');

  let THREE, RoomEnvironment;
  try {
    THREE = await import('three');
    ({ RoomEnvironment } = await import('/vendor/RoomEnvironment.js'));
  } catch {
    document.documentElement.classList.add('no-webgl');
    return;
  }
  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: 'high-performance' });
  } catch {
    document.documentElement.classList.add('no-webgl');
    return;
  }
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;

  const scene = new THREE.Scene();
  const pmrem = new THREE.PMREMGenerator(renderer);
  scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  const camera = new THREE.PerspectiveCamera(34, 1, 0.1, 100);
  camera.position.set(0, 0, 12);

  const key = new THREE.DirectionalLight(0xffffff, 1.6);
  key.position.set(3, 5, 6);
  scene.add(key, new THREE.AmbientLight(0xffffff, 0.35));

  // A rounded tile with real thickness, like an app icon made of glass.
  const shape = new THREE.Shape();
  const s = 0.5, r = 0.22;
  shape.moveTo(-s + r, -s);
  shape.lineTo(s - r, -s); shape.quadraticCurveTo(s, -s, s, -s + r);
  shape.lineTo(s, s - r); shape.quadraticCurveTo(s, s, s - r, s);
  shape.lineTo(-s + r, s); shape.quadraticCurveTo(-s, s, -s, s - r);
  shape.lineTo(-s, -s + r); shape.quadraticCurveTo(-s, -s, -s + r, -s);
  const depth = 0.14;
  const body = new THREE.ExtrudeGeometry(shape, { depth, bevelEnabled: true, bevelThickness: 0.04, bevelSize: 0.035, bevelSegments: 6, curveSegments: 18 });
  body.translate(0, 0, -depth / 2);
  const face = new THREE.PlaneGeometry(0.98, 0.98);
  const loader = new THREE.TextureLoader();

  const group = new THREE.Group();
  scene.add(group);
  const tiles = ids.map((id, i) => {
    const tex = loader.load(`/icons/${id}.png`);
    tex.colorSpace = THREE.SRGBColorSpace;
    tex.anisotropy = 8;
    const tile = new THREE.Group();
    const glass = new THREE.Mesh(body, new THREE.MeshPhysicalMaterial({ color: 0x17171f, roughness: 0.28, metalness: 0.15, clearcoat: 1, clearcoatRoughness: 0.12 }));
    const front = new THREE.Mesh(face, new THREE.MeshBasicMaterial({ map: tex, transparent: true }));
    front.position.z = depth / 2 + 0.042;
    tile.add(glass, front);
    tile.userData = { id, index: i, scale: new Spring(1, { response: 0.35 }), phase: (i / ids.length) * Math.PI * 2 };
    group.add(tile);
    return tile;
  });

  // Pointer parallax, scroll progress and hover all drive springs; the render loop reads them.
  const tiltX = new Spring(0), tiltY = new Spring(0), scrollP = new Spring(0, { response: 0.5 });
  const pointer = new THREE.Vector2(9, 9);
  const ray = new THREE.Raycaster();
  let hovered = null;
  let w = 1, h = 1, rx = 4, ry = 2.4;

  const resize = () => {
    w = canvas.clientWidth; h = canvas.clientHeight;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    const halfH = camera.position.z * Math.tan((camera.fov * Math.PI) / 360);
    const halfW = halfH * camera.aspect;
    const tall = camera.aspect < 0.9;
    rx = Math.min(halfW * (tall ? 0.92 : 0.9), 7.2);
    ry = halfH * (tall ? 0.9 : 0.84);
    const size = tall ? Math.max(0.5, halfW / 4.2) : Math.min(1.25, Math.max(0.72, halfW / 5.2));
    tiles.forEach((t) => { t.userData.base = size; });
  };
  new ResizeObserver(resize).observe(canvas);
  resize();

  addEventListener('pointermove', (e) => {
    if (reduceMotion) return;
    tiltY.target = (e.clientX / innerWidth - 0.5) * 0.35;
    tiltX.target = (e.clientY / innerHeight - 0.5) * 0.22;
  }, { passive: true });
  canvas.parentElement.addEventListener('pointermove', (e) => {
    const rect = canvas.getBoundingClientRect();
    pointer.set(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1);
  });
  canvas.parentElement.addEventListener('pointerleave', () => pointer.set(9, 9));
  canvas.parentElement.addEventListener('click', (e) => {
    if (!hovered || e.target.closest('a, button')) return;
    const id = hovered.userData.id;
    document.querySelector(id === 'studio' ? '#telecharger' : `#dl-${id}`)?.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'center' });
  });
  addEventListener('scroll', () => { scrollP.target = Math.min(scrollY / innerHeight, 1.2); }, { passive: true });

  let onScreen = true;
  new IntersectionObserver(([e]) => { onScreen = e.isIntersecting; }).observe(canvas);

  const names = { studio: 'Ethone Studio · le lanceur' };
  APPS.forEach((a) => { names[a.id] = `${a.name} · ${a.tagline.replace(/\.$/, '')}`; });

  let last = performance.now();
  let spin = 0;
  const loop = (t) => {
    requestAnimationFrame(loop);
    const dt = Math.min((t - last) / 1000, 1 / 30);
    last = t;
    if (!onScreen || document.hidden) return;
    if (!reduceMotion) spin += dt * 0.09;
    tiltX.step(dt); tiltY.step(dt); scrollP.step(dt);
    const p = Math.max(0, scrollP.value);

    group.rotation.x = tiltX.value - p * 0.25;
    group.rotation.y = tiltY.value + p * 0.4;
    group.position.y = p * 1.6;
    camera.position.z = 12 + p * 3;

    tiles.forEach((tile) => {
      const d = tile.userData;
      const a = d.phase + spin;
      tile.position.set(Math.cos(a) * rx * (1 + p * 0.35), Math.sin(a) * ry * (1 + p * 0.25), -1.4 + Math.sin(a * 2 + spin) * 0.8);
      const bob = reduceMotion ? 0 : Math.sin(t / 1400 + d.phase * 3) * 0.06;
      tile.position.y += bob;
      tile.rotation.y = -group.rotation.y * 0.6 + (reduceMotion ? 0 : Math.sin(t / 2200 + d.phase) * 0.18);
      tile.rotation.x = -group.rotation.x * 0.6 + (reduceMotion ? 0 : Math.cos(t / 2600 + d.phase) * 0.12);
      d.scale.target = tile === hovered ? 1.18 : 1;
      const k = d.scale.step(dt) * d.base;
      tile.scale.setScalar(k);
    });

    ray.setFromCamera(pointer, camera);
    const hit = ray.intersectObjects(tiles.map((t) => t.children[0]), false)[0];
    const now = hit ? hit.object.parent : null;
    if (now !== hovered) {
      hovered = now;
      canvas.parentElement.style.cursor = hovered ? 'pointer' : '';
      label.textContent = hovered ? names[hovered.userData.id] : '';
    }
    canvas.style.opacity = String(Math.max(0, 1 - p * 0.85));
    renderer.render(scene, camera);
  };
  requestAnimationFrame(loop);
}

buildCards();
buildFormats();
buildDownloads();
reveals();
counters();
navMaterial();
paletteDemo();
hero();
