const treeForms = ['Rounded', 'Pointed', 'Leaning', 'Forked', 'Spire', 'Umbrella', 'Vase', 'Fan', 'Cluster', 'Crescent', 'Tiered', 'Wind swept', 'Three lobed', 'Cut crown', 'Split ridge', 'Cushion'];
const treeFootprints = ['Compact', 'Wide', 'Deep', 'Broad'];
const treeSpecies = ['Oak', 'Spruce', 'Juniper', 'Maple', 'Spruce', 'Oak', 'Aspen', 'Aspen', 'Oak', 'Juniper', 'Spruce', 'Juniper', 'Maple', 'Maple', 'Aspen', 'Oak'];
const cactusSpecies = ['Saguaro', 'Organ pipe', 'Prickly pear', 'Golden barrel'];
const rockForms = ['Tumbled granite', 'Sandstone ledge', 'Basalt shards', 'Pale scree'];
const shrubForms = ['Sagebrush', 'Bunchgrass', 'Saltbush', 'Flowering yucca'];
const wetlandForms = ['Bulrush', 'Cattail', 'Iris', 'Willow scrub'];
const lighthouseTypes = [
  ['Masonry tower', 'Tapered shaft on a solid coastal footing'],
  ['Keeper tower', 'Square shaft and compact lantern room'],
  ['Steel lattice', 'Open bracing with a compact grounded footprint'],
];
const locomotives = [
  ['Wide cab', 'Container, vehicle, and refrigerated trains'],
  ['Heavy haul', 'Aggregate, mineral, and agricultural trains'],
  ['Road switcher', 'Mixed goods and longer branch runs'],
  ['High hood', 'Forest products trains'],
  ['Diesel transfer', 'Short harbor or mill runs'],
  ['Battery transfer', 'Occasional short liquid-terminal runs'],
];
const shipTypes = [
  ['Freighter', 'Cargo bays and an aft bridge for shipping goods'],
  ['Ferry', 'Wide hull and passenger decks for coastal crossings'],
  ['Sailboat', 'Sail rig and a small cabin for recreational sailing'],
  ['Fishing boat', 'Forward wheelhouse and a working deck for hauling catch'],
  ['Tug', 'Compact hull and tall wheelhouse for maneuvering ships'],
];

const [wagonMetadata, vesselMetadata] = await Promise.all(
  ['wagons', 'vessels'].map(name => fetch(`assets/artwork/${name}.json`)
    .then(response => response.ok ? response.json() : [])
    .catch(() => [])),
);

const categories = [
  { title: 'Trees', label: 'Tree', atlas: 'trees.png', family: index => treeSpecies[index & 15] },
  { title: 'Rocks', label: 'Rock', atlas: 'rocks.png', family: index => rockForms[index >> 6], traits: ['Width', 'Depth', 'Relief'] },
  { title: 'Bushes and grasses', label: 'Shrub', atlas: 'bushes.png', family: index => shrubForms[index >> 6], traits: ['Width', 'Height', 'Spread'] },
  { title: 'Cacti', label: 'Cactus', atlas: 'cacti.png', family: index => cactusSpecies[index >> 6],
    traits: ['Girth', 'Height', 'Spread'] },
  { title: 'Freight wagons', label: 'Freight wagon', atlas: 'wagons.png', columns: 10, rows: 25,
    family: index => wagonMetadata[index]?.family,
    detail: index => {
      const wagon = wagonMetadata[index];
      if (!wagon) return '';
      const kind = wagon.kind === wagon.family ? '' : wagon.kind;
      return [kind, wagon.cargo, wagon.equipment].filter(Boolean).join(' · ');
    },
    tile: index => [Math.floor(index / 25) % 2 * 5 + index % 5,
      Math.floor(index / 50) * 5 + Math.floor(index % 25 / 5)] },
  { title: 'Vessels', label: 'Vessel', atlas: 'vessels.png', columns: 10, rows: 25,
    family: index => shipTypes[Math.floor(index / 50)][0],
    detail: index => [vesselMetadata[index]?.configuration, shipTypes[Math.floor(index / 50)][1]]
      .filter(Boolean).join(' · ') },
  { title: 'Wetland plants', label: 'Wetland plant', atlas: 'wetlands.png',
    family: index => wetlandForms[index >> 6], traits: ['Width', 'Height', 'Spread'] },
  { title: 'Lighthouses', label: 'Lighthouse', atlas: 'lighthouses.png', columns: 3, rows: 1,
    family: index => lighthouseTypes[index][0], detail: index => lighthouseTypes[index][1] },
  { title: 'Locomotives', label: 'Locomotive', atlas: 'locomotives.png', columns: 3, rows: 2,
    family: index => locomotives[index][0], detail: index => locomotives[index][1] },
];
const tabs = document.querySelector('#artwork-categories');
const families = document.querySelector('#artwork-family');
const grid = document.querySelector('#artwork-grid');
const count = document.querySelector('#artwork-count');
const viewer = document.querySelector('#viewer');
const viewerMedia = document.querySelector('#viewer-media');
const viewerTitle = document.querySelector('#viewer-title');
const viewerInfo = document.querySelector('#viewer-info');
let activeCategory = categories[0];

function variantCount(category) {
  return (category.columns || 16) * (category.rows || 16);
}

function sprite(category, index) {
  const image = document.createElement('span');
  const columns = category.columns || 16;
  const rows = category.rows || 16;
  const [x, y] = category.tile?.(index) ?? [index % columns, Math.floor(index / columns)];
  image.className = 'artwork-sprite';
  image.style.backgroundImage = `url("assets/artwork/${category.atlas}")`;
  image.style.backgroundSize = `${columns * 100}% ${rows * 100}%`;
  image.style.backgroundPosition = `${x * 100 / Math.max(1, columns - 1)}% ${y * 100 / Math.max(1, rows - 1)}%`;
  image.setAttribute('aria-hidden', 'true');
  return image;
}

function variantDetails(category, index) {
  if (category.detail) return category.detail(index);
  if (category.label === 'Tree') {
    return [treeForms[index & 15], treeFootprints[(index >> 4) & 3],
      `Height ${(index >> 6) + 1} of 4`].join(' · ');
  }
  return category.traits?.map((trait, axis) => `${trait} ${((index >> (axis * 2)) & 3) + 1} of 4`).join(' · ') || '';
}

function render(category) {
  const entries = document.createDocumentFragment();
  const selected = families.value;
  let displayed = 0;
  for (let index = 0; index < variantCount(category); index++) {
    const family = category.family?.(index);
    if (selected && family !== selected) continue;
    displayed++;
    const entry = document.createElement('li');
    const button = document.createElement('button');
    const title = `${family || category.label} ${String(index + 1).padStart(3, '0')}`;
    button.type = 'button';
    button.className = 'artwork-tile';
    button.setAttribute('aria-label', title);
    const caption = document.createElement('span');
    caption.className = 'artwork-number';
    caption.textContent = title;
    button.append(sprite(category, index), caption);
    button.addEventListener('click', () => {
      viewerTitle.textContent = title;
      viewerInfo.textContent = variantDetails(category, index);
      viewerInfo.hidden = !viewerInfo.textContent;
      viewerMedia.replaceChildren(sprite(category, index));
      viewer.showModal();
    });
    entry.append(button);
    entries.append(entry);
  }
  grid.setAttribute('aria-label', selected || `${category.label} variants`);
  grid.replaceChildren(entries);
  count.textContent = `${displayed} ${displayed === 1 ? 'variant' : 'variants'}`;
}

function selectCategory(category) {
  activeCategory = category;
  const names = category.family
    ? new Set(Array.from({ length: variantCount(category) }, (_, index) => category.family(index)))
    : [];
  families.replaceChildren(new Option('All families', ''), ...Array.from(names, name => new Option(name, name)));
  families.closest('label').hidden = !category.family;
  for (const button of tabs.children) {
    button.setAttribute('aria-pressed', String(button.textContent === category.title));
  }
  render(category);
}

families.addEventListener('change', () => render(activeCategory));
for (const category of categories) {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = category.title;
  button.addEventListener('click', () => selectCategory(category));
  tabs.append(button);
}
selectCategory(activeCategory);
