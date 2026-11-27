const planeAssets = [
  ['Bush Plane', 'Low and deliberate', 'Climbs early and banks through mountain passes.'],
  ['Racer', 'Quick and agile', 'Sweeps into tighter lowland turns and finds dry exits from islands.'],
  ['Sport Jet', 'Fast and measured', 'Gradual pitch changes, shallow banks, and occasional coastal passes.'],
  ['Ultralight', 'Slow and low', 'Can briefly follow a passing train, with a bias toward passengers.'],
  ['Glider', 'Quiet and unhurried', 'Circles strong thermals and gains lift on windward ridges.'],
  ['F/A-18 Fighter', 'Fast and responsive', 'Two- or four-jet formations and twin orange afterburners.'],
  ['SR-71 Blackbird', 'Very high, fast and steady', 'Purple reheat and a level tanker rendezvous below cruise.'],
  ['Bald Eagle', 'Lowest and slowest', 'Measured wingbeats, soaring lift and occasional train follows.'],
];

const arrangement = await fetch('./assets/gallery-order.json').then(response => response.json());
const planeOrder = arrangement.aircraft;
const planes = planeOrder.map(entry => planeAssets[entry.asset]);

const events = [
  ['Blackbird refueling', 'Rare', '24–29 s · ⅔ chance per 21 min'],
  ['Blackbird purple reheat', 'Rare', '4.8–7.6 s · ⅔ chance per 7 min'],
  ['Fighter orange afterburner', 'Rare', '4.1–6.3 s · ½ chance per 6 min'],
  ['Fighter formation', 'Rare', '2 or 4 wingmates · 11.5–18.25 s · 2 in 7 chance per 9 min'],
  ['Sport Jet coastal pass', 'Conditional', '14–19 s · eligible coastal flybys'],
  ['Glider thermal circle', 'Conditional', 'Strong lift · up to 15.5 s per orbit sequence'],
  ['Eagle and ultralight train follow', 'Conditional', 'Passenger trains favored when a safe approach is available'],
  ['Meteor', 'Rare', '0.75–1.4 s · 2 in 7 chance per 225 s · clear nights'],
];

const categories = ['All', 'Sky & Weather', 'Encounters', 'Aircraft', 'Biomes', 'Built World', 'Interface'];
const featuredIds = arrangement.featured;
const qs = selector => document.querySelector(selector);
const cards = qs('#cards');
const dialog = qs('#viewer');
const filters = ['#search', '#aircraft', '#biome', '#rarity'].map(qs);
let filterCategory = 'All';
let records = [];

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function option(text) {
  const node = element('option', text);
  node.value = text;
  return node;
}

function encounter(item) {
  const title = item.title.toLowerCase();
  if (/refuel|tanker/.test(title)) return events[0];
  if (/blackbird.*reheat|sr.71 afterburner/.test(title)) return events[1];
  if (/fighter orange afterburner/.test(title)) return events[2];
  if (/formation/.test(title)) return events[3];
  if (/sport jet.*coast/.test(title)) return events[4];
  if (/thermal/.test(title)) return events[5];
  if (/train follow/.test(title)) return events[6];
  if (/meteor|shooting star/.test(title)) return events[7];
  return null;
}

function show(item) {
  qs('#viewer-title').textContent = item.title;
  const media = element(item.format === 'Video' ? 'video' : 'img');
  media.src = item.src;
  if (media instanceof HTMLVideoElement) {
    media.controls = true;
    media.autoplay = true;
    media.playsInline = true;
    media.preload = 'metadata';
    media.poster = item.poster;
  } else {
    media.alt = item.title;
  }
  qs('#viewer-media').replaceChildren(media);
  const info = item.note || encounter(item)?.[2];
  const details = qs('#viewer-info');
  details.textContent = [item.aircraft, item.biome, info].filter(Boolean).join(' · ');
  details.hidden = !details.textContent;
  if (!dialog.open) dialog.showModal();
}

dialog.addEventListener('close', () => qs('#viewer-media').replaceChildren());
dialog.addEventListener('click', event => {
  if (event.target === dialog) {
    const box = dialog.getBoundingClientRect();
    if (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom) dialog.close();
  }
});

function createCard(item, preview = item.poster) {
  const button = element('button', '', 'card');
  button.type = 'button';
  const image = element('img');
  image.src = preview;
  image.alt = '';
  image.loading = 'lazy';
  const group = element('span', '', 'card-text');
  const badge = [item.format === 'Video' ? '▶ Play' : item.format, item.biome || item.category].join(' · ');
  group.append(element('small', badge), element('b', item.title));
  const detail = item.note || encounter(item)?.[2];
  if (detail) group.append(element('em', detail));
  button.append(image, group);
  button.addEventListener('click', () => show(item));
  return button;
}

function renderGallery() {
  const [search, plane, biome, rarity] = filters;
  const needle = search.value.trim().toLowerCase();
  const filtered = records.filter(item =>
    (filterCategory === 'All' || item.category === filterCategory) &&
    (!plane.value || item.aircraft === plane.value) &&
    (!biome.value || item.biome === biome.value) &&
    (!rarity.value || (encounter(item)?.[1] || 'Routine') === rarity.value) &&
    (!needle || [item.title, item.category, item.aircraft, item.biome, item.note].join(' ').toLowerCase().includes(needle))
  );
  if (filtered.length) cards.replaceChildren(...filtered.map(item => createCard(item)));
  else cards.replaceChildren(element('p', 'No captures match these filters.'));
  qs('#media-count').textContent = `${filtered.length} ${filtered.length === 1 ? 'visual' : 'visuals'}`;
  for (const tab of qs('#categories').children) {
    tab.setAttribute('aria-pressed', String(tab.textContent === filterCategory));
  }
}

function browsePlane(name) {
  filters[0].value = '';
  filters[1].value = name;
  filters[2].value = '';
  filters[3].value = '';
  filterCategory = 'All';
  renderGallery();
  qs('#gallery').scrollIntoView();
}

function coverage() {
  for (const [name, style, signature] of planes) {
    const row = element('tr');
    const heading = element('th', name);
    heading.scope = 'row';
    const visuals = records.filter(item => item.aircraft === name);
    const count = element('td');
    const button = element('button', `${visuals.length} visuals`, 'button');
    button.type = 'button';
    button.addEventListener('click', () => browsePlane(name));
    count.append(button);
    row.append(heading, element('td', style), element('td', signature), count);
    qs('#aircraft-coverage').append(row);
  }
  for (const [name, frequency, timing] of events) {
    const item = element('li');
    item.append(element('strong', name), element('span', ` · ${frequency} · ${timing}`));
    qs('#encounter-rates').append(item);
  }
}

async function get(file) {
  const response = await fetch(file);
  if (!response.ok) throw new Error(`Could not load ${file}`);
  return response;
}

for (const name of categories) {
  const button = element('button', name);
  button.type = 'button';
  button.addEventListener('click', () => { filterCategory = name; renderGallery(); });
  qs('#categories').append(button);
}
qs('#aircraft').append(...planes.map(([name])=>option(name)));
for (const filter of filters) filter.addEventListener(filter.type === 'search' ? 'input' : 'change', renderGallery);
try {
  records = await (await get('./assets/media.json')).json();
  const biomeNames = [...new Set(records.map(item => item.biome).filter(Boolean))].sort();
  qs('#biome').append(...biomeNames.map(option));
  const figures = planeOrder.map(({name,asset:index})=>({id:`turntable-${index}`,title:`${name} turntable`,category:'Aircraft',
    aircraft:name,biome:'',format:'GIF',poster:`assets/turntable-${index}.png`,src:`assets/turntable-${index}.gif`,
    note:'Animated in-game model'}));
  const pickers = planeOrder.map(({name,asset:index})=>({id:`picker-${index}`,title:`${name} picker`,category:'Interface',
    aircraft:name,biome:'',format:'Still',poster:`assets/picker-${index}.png`,src:`assets/picker-${index}.png`}));
  const featured = featuredIds.map(id => records.find(item => item.id === id));
  const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)');
  const renderFeatured = () => {
    qs('#featured').replaceChildren(...featured.map(item => createCard(item,
      reduceMotion.matches ? item.poster : `assets/readme/${item.id}.gif`)));
  };
  renderFeatured();
  reduceMotion.addEventListener('change', renderFeatured);
  qs('#aircraft-artwork-cards').replaceChildren(...planeOrder.map((plane, index) => {
    const group = element('div', '', 'aircraft-pair');
    group.append(element('h3', plane.name), createCard(figures[index], figures[index].src), createCard(pickers[index]));
    return group;
  }));
  try {
    const momentIndex = await (await get('./assets/artwork/flight-moments.tsv')).text();
    const moments = new Map(momentIndex.trim().split('\n').map(row => {
      const [asset, , ...names] = row.split('\t');
      return [Number(asset), names];
    }));
    qs('#flight-moment-cards').replaceChildren(...planeOrder.map(({ name, asset }) => {
      const labels = moments.get(asset).join(' · ');
      const card = element('article', '', 'moment-card');
      const link = element('a', '');
      link.href = `assets/artwork/flight-moments-${asset}.gif`;
      const image = element('img', '');
      image.loading = 'lazy';
      image.src = reduceMotion.matches ? `assets/artwork/flight-moments-${asset}.png` : link.href;
      image.alt = `${name}. ${labels}`;
      link.append(image);
      card.append(element('h3', name), element('p', labels), link);
      return card;
    }));
  } catch (error) {
    qs('#flight-moment-cards').replaceChildren(element('p', error.message));
  }
  records.reverse();
  renderGallery();
  coverage();
} catch (error) {
  cards.replaceChildren(element('p', error.message));
}
