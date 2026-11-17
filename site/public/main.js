// Ethone Studio site: 3D hero (three.js), spring-driven motion, Ctrl+K demo, real downloads.
// Springs follow Apple's damping/response model: they start from the current value and can be
// redirected at any moment, so nothing ever jumps or locks out the pointer.

const RELEASE = 'https://github.com/Rub19/ethone-studio/releases/latest/download/';
const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

const U = (id, w = 1400) => `https://images.unsplash.com/${id}?w=${w}&q=72&auto=format&fit=crop`;

const APPS = [
  { id: 'pixel', name: 'Pixel', what: 'Retouche photo', c: '255 77 109', t: '214 30 70', head: ['Il n\'y a qu\'un', 'Pixel.'], text: 'Retouche, montage photo et compositing par calques. Tes PSD s\'ouvrent calque par calque, et tes réflexes Photoshop marchent ici.', tags: ['Calques et masques', '38 outils', 'Fichiers PSD'], photo: 'photo-1516575150278-77136aed6920', size: '23 Mo' },
  { id: 'lumen', name: 'Lumen', what: 'Photo RAW', c: '47 214 154', t: '10 140 95', head: ['Tes photos,', 'développées.'], text: 'Développement RAW, catalogue et looks pellicule. Tes préréglages Lightroom et tes LUT fonctionnent tels quels.', tags: ['RAW', 'Looks pellicule', 'Préréglages XMP'], photo: 'photo-1604223190546-a43e4c7f29d7', size: '23 Mo' },
  { id: 'vector', name: 'Vector', what: 'Dessin vectoriel', c: '255 176 0', t: '190 110 0', head: ['Net à toutes', 'les tailles.'], text: 'Logos, affiches, illustrations et typographie en vectoriel, avec 2 359 icônes prêtes à l\'emploi.', tags: ['2 359 icônes', 'Nuanciers ASE', 'SVG optimisé'], photo: 'photo-1767706508363-b2a729df06fb', size: '23 Mo' },
  { id: 'cut', name: 'Cut', what: 'Montage vidéo', c: '184 107 255', t: '120 50 210', head: ['Le montage,', 'sans détour.'], text: 'Montage multipiste, transitions et titres. Tu récupères les médias de tes projets Premiere en un clic.', tags: ['Multipiste', 'Transitions', 'Médias Premiere'], photo: 'photo-1515634928627-2a4e0dae3ddf', size: '22 Mo' },
  { id: 'motion', name: 'Motion', what: 'Animation', c: '61 190 245', t: '0 120 190', head: ['Fais bouger', 'tes idées.'], text: 'Images clés, courbes et titres animés, dans tous les formats, du 16:9 au 9:16.', tags: ['Images clés', 'Courbes', 'Formats 9:16'], photo: 'photo-1490049350474-498de43bc885', size: '23 Mo' },
  { id: 'page', name: 'Page', what: 'Mise en page', c: '255 122 61', t: '210 80 20', head: ['Mets tes idées', 'en page.'], text: 'Magazines, affiches et documents avec gabarits. Tes fichiers InDesign IDML s\'ouvrent, l\'export PDF est prêt pour l\'imprimeur.', tags: ['Gabarits', 'InDesign IDML', 'Export PDF'], photo: 'photo-1588775161074-a5a92eaa9a37', size: '24 Mo' },
  { id: 'folio', name: 'Folio', what: 'PDF', c: '242 242 242', t: '14 14 14', head: ['Tous tes PDF,', 'en ordre.'], text: 'Lis, annote, signe et réorganise tes PDF. La reconnaissance de texte rend les scans consultables.', tags: ['OCR', 'Signature', 'Organisation'], photo: 'photo-1695388474402-ed805a890d8d', size: '25 Mo' },
  { id: 'sound', name: 'Sound', what: 'Audio', c: '140 140 255', t: '80 70 220', head: ['Un son propre,', 'en trois clics.'], text: 'Coupe, nettoie et transforme l\'audio avec le spectrogramme et 311 bruitages libres de droits.', tags: ['311 bruitages', 'Spectrogramme', 'Effets voix'], photo: 'photo-1541592553160-82008b127ccb', size: '24 Mo' },
];

const FORMATS = [
  { ext: 'PSD', app: 'Pixel', what: 'calque par calque', c: '255 77 109' },
  { ext: 'AI', app: 'Folio', what: 'compatible PDF', c: '242 242 242' },
  { ext: 'XMP', app: 'Lumen', what: 'préréglages Lightroom', c: '47 214 154' },
  { ext: 'CUBE', app: 'Lumen', what: 'LUT 3D', c: '47 214 154' },
  { ext: 'ASE', app: 'Vector', what: 'nuanciers Adobe', c: '255 176 0' },
  { ext: 'ACO', app: 'Pixel', what: 'nuanciers Photoshop', c: '255 77 109' },
  { ext: 'IDML', app: 'Page', what: 'documents InDesign', c: '255 122 61' },
  { ext: 'PRPROJ', app: 'Cut', what: 'médias Premiere', c: '184 107 255' },
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

function buildStory() {
  const panels = document.getElementById('story-panels');
  const rail = document.getElementById('story-rail');
  document.getElementById('story').style.setProperty('--n', APPS.length);
  APPS.forEach((a, i) => {
    const el = document.createElement('article');
    el.className = 'panel';
    el.style.setProperty('--a', a.t);
    el.innerHTML = `
      <div class="panel-copy">
        <div class="panel-head"><img src="/icons/${a.id}.png" alt="" width="52" height="52"><span>Ethone ${a.name}<small>${a.what}</small></span><b class="panel-num">${String(i + 1).padStart(2, '0')} / ${String(APPS.length).padStart(2, '0')}</b></div>
        <h3>${a.head[0]} <i>${a.head[1]}</i></h3>
        <p>${a.text}</p>
        <ul>${a.tags.map((t) => `<li>${t}</li>`).join('')}</ul>
        <a class="button" href="${RELEASE}${a.name}-Setup.exe">Installer ${a.name}</a>
      </div>
      <div class="panel-art">
        <div class="panel-photo"><img src="${U(a.photo)}" alt="" loading="lazy"></div>
        <div class="panel-shot"><img src="/shots/${a.id}.webp" alt="${a.name} en cours d'utilisation" loading="lazy"></div>
      </div>`;
    panels.append(el);
    const b = document.createElement('button');
    b.type = 'button';
    b.setAttribute('aria-label', a.name);
    b.innerHTML = `<img src="/icons/${a.id}.png" alt="">`;
    b.addEventListener('click', () => {
      const track = document.getElementById('story');
      const span = track.offsetHeight - innerHeight;
      const y = track.getBoundingClientRect().top + scrollY + ((i + 0.5) / APPS.length) * span;
      scrollTo({ top: y, behavior: reduceMotion ? 'auto' : 'smooth' });
    });
    rail.append(b);
  });
}

/* ---------- Scroll scenes ---------- */

// Scroll positions feed springs, so a wheel step glides instead of jumping, and every frame
// starts from where the element is on screen.
function scenes() {
  const clamp = (v, a = 0, b = 1) => Math.min(b, Math.max(a, v));
  const story = document.querySelector('.story');
  const track = document.getElementById('story');
  const panels = [...document.querySelectorAll('.panel')];
  const railBtns = [...document.querySelectorAll('.story-rail button')];
  const show = document.getElementById('showcase');
  const win = document.getElementById('show-window');
  const copy = document.getElementById('show-copy');
  const speeds = [...document.querySelectorAll('[data-speed]')];
  const flatMq = matchMedia('(max-width: 860px)');
  const showP = new Spring(0, { response: 0.32 });
  const storyP = new Spring(0, { response: 0.38 });
  let running = false, last = 0;

  const progress = (el) => {
    const r = el.getBoundingClientRect();
    return clamp(-r.top / Math.max(1, r.height - innerHeight));
  };
  const flat = () => reduceMotion || flatMq.matches;

  const applyFlat = () => {
    const f = flat();
    story.classList.toggle('flat', f);
    if (f) panels.forEach((p) => {
      p.style.visibility = ''; p.style.pointerEvents = '';
      p.querySelectorAll('.panel-copy, .panel-art, .panel-shot').forEach((e) => { e.style.transform = ''; e.style.opacity = ''; });
    });
  };

  const frame = (t) => {
    const dt = Math.min((t - last) / 1000, 1 / 30);
    last = t;
    showP.target = progress(show);
    storyP.target = progress(track);
    showP.step(dt); storyP.step(dt);

    if (!reduceMotion) {
      const e = 1 - (1 - clamp(showP.value / 0.75)) ** 3;
      win.style.transform = `perspective(1600px) translateY(${(1 - e) * 90}px) rotateX(${(1 - e) * 30}deg) scale(${0.74 + 0.26 * e})`;
      const fade = clamp((showP.value - 0.45) / 0.35);
      copy.style.transform = `translateY(${-fade * 60}px)`;
      copy.style.opacity = String(1 - fade);
    }

    if (!flat()) {
      const n = panels.length;
      const cur = clamp(storyP.value * n - 0.5, 0, n - 1);
      panels.forEach((p, i) => {
        const raw = cur - i;
        const d = Math.sign(raw) * Math.max(0, Math.abs(raw) - 0.18) / 0.82;
        const ad = Math.abs(d);
        const c = p.firstElementChild, art = c.nextElementSibling, shot = art.lastElementChild;
        p.style.visibility = ad < 1 ? 'visible' : 'hidden';
        p.style.pointerEvents = ad < 0.5 ? 'auto' : 'none';
        c.style.opacity = String(clamp(1 - ad * 1.8));
        c.style.transform = `translateY(${-d * 70}px)`;
        art.style.opacity = String(clamp(1 - ad * 1.3));
        art.style.transform = `translateY(${-d * 140}px) scale(${1 - ad * 0.06})`;
        shot.style.transform = `translateY(${-d * 90}px)`;
      });
      const on = Math.round(cur);
      railBtns.forEach((b, i) => b.classList.toggle('on', i === on));
    }

    if (!reduceMotion) {
      for (const el of speeds) {
        const r = el.getBoundingClientRect();
        el.style.translate = `0 ${((r.top + r.height / 2) - innerHeight / 2) * Number(el.dataset.speed)}px`;
      }
    }

    if (showP.settled && storyP.settled) { running = false; return; }
    requestAnimationFrame(frame);
  };
  const kick = () => {
    if (running) return;
    running = true;
    last = performance.now();
    requestAnimationFrame(frame);
  };
  flatMq.addEventListener('change', () => { applyFlat(); kick(); });
  applyFlat();
  addEventListener('scroll', kick, { passive: true });
  addEventListener('resize', kick);
  kick();
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
    const glass = new THREE.Mesh(body, new THREE.MeshPhysicalMaterial({ color: 0x1a1a1a, roughness: 0.28, metalness: 0.15, clearcoat: 1, clearcoatRoughness: 0.12 }));
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
  let w = 1, h = 1, rx = 4, ry = 2.4, offX = 0, offY = 0;

  const resize = () => {
    w = canvas.clientWidth; h = canvas.clientHeight;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    const halfH = camera.position.z * Math.tan((camera.fov * Math.PI) / 360);
    const halfW = halfH * camera.aspect;
    const tall = camera.aspect < 1.1;
    rx = tall ? halfW * 0.78 : Math.min(halfW * 0.3, 4.4);
    ry = tall ? halfH * 0.3 : halfH * 0.7;
    offX = tall ? 0 : halfW * 0.6;
    offY = tall ? halfH * 0.45 : 0;
    const size = tall ? Math.max(0.55, halfW / 4) : Math.min(1.35, Math.max(0.8, halfW / 5));
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
  APPS.forEach((a) => { names[a.id] = `${a.name} · ${a.what}`; });

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
    group.position.x = offX;
    group.position.y = offY + p * 1.6;
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

buildStory();
scenes();
buildFormats();
buildDownloads();
reveals();
counters();
navMaterial();
paletteDemo();
hero();
