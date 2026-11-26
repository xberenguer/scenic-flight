const treeForms = ['Rounded', 'Pointed', 'Leaning', 'Forked', 'Spire', 'Umbrella', 'Vase', 'Fan', 'Cluster', 'Crescent', 'Tiered', 'Wind swept', 'Three lobed', 'Cut crown', 'Split ridge', 'Cushion'];
const treeFootprints = ['Compact', 'Wide', 'Deep', 'Broad'];
const wagonTypes = ['Container', 'Boxcar', 'Hopper', 'Tanker', 'Loaded flatcar'];
const categories = [
  { title: 'Trees', label: 'Tree', atlas: 'trees.png' },
  { title: 'Rocks', label: 'Rock', atlas: 'rocks.png', traits: ['Width', 'Depth', 'Relief', 'Crown'] },
  { title: 'Bushes', label: 'Bush', atlas: 'bushes.png', traits: ['Width', 'Height', 'Spread', 'Crown'] },
  { title: 'Cacti', label: 'Cactus', atlas: 'cacti.png', traits: ['Girth', 'Height', 'Arm reach', 'Arm arrangement'] },
  { title: 'Freight Wagons', label: 'Freight wagon', atlas: 'wagons.png', columns: 5 },
];
const tabs = document.querySelector('#artwork-categories');
const grid = document.querySelector('#artwork-grid');
const count = document.querySelector('#artwork-count');
const viewer = document.querySelector('#viewer');
const viewerMedia = document.querySelector('#viewer-media');
const viewerTitle = document.querySelector('#viewer-title');
const viewerInfo = document.querySelector('#viewer-info');

function sprite(category, index) {
  const image = document.createElement('span');
  const atlasColumns = category.columns || 16;
  const x = index % atlasColumns;
  const y = Math.floor(index / atlasColumns);
  image.className = 'artwork-sprite';
  image.style.backgroundImage = `url("assets/artwork/${category.atlas}")`;
  image.style.backgroundSize = `${atlasColumns * 100}% ${atlasColumns * 100}%`;
  image.style.backgroundPosition = `${x * 100 / (atlasColumns - 1)}% ${y * 100 / (atlasColumns - 1)}%`;
  image.setAttribute('aria-hidden', 'true');
  return image;
}

function render(category) {
  const entries = document.createDocumentFragment();
  const variantCount = (category.columns || 16) ** 2;
  for (let index = 0; index < variantCount; index++) {
    const entry = document.createElement('li');
    const button = document.createElement('button');
    const number = String(index).padStart(3, '0');
    const title = category.columns === 5
      ? `${wagonTypes[Math.floor(index / 5)]} ${index % 5 + 1}`
      : `${category.label} ${number}`;
    button.type = 'button';
    button.className = 'artwork-tile';
    button.setAttribute('aria-label', title);
    const caption = document.createElement('span');
    caption.className = 'artwork-number';
    caption.textContent = number;
    button.append(sprite(category, index), caption);
    button.addEventListener('click', () => {
      viewerTitle.textContent = title;
      viewerInfo.textContent = category.label === 'Tree'
        ? [treeForms[index & 15], treeFootprints[(index >> 4) & 3], `Height ${(index >> 6) + 1} of 4`].join(' · ')
        : category.traits?.map((trait, axis) => `${trait} ${((index >> (axis * 2)) & 3) + 1} of 4`).join(' · ') || '';
      viewerInfo.hidden = !viewerInfo.textContent;
      viewerMedia.replaceChildren(sprite(category, index));
      viewer.showModal();
    });
    entry.append(button);
    entries.append(entry);
  }
  grid.setAttribute('aria-label', `${category.label} variants`);
  grid.replaceChildren(entries);
  count.textContent = `${variantCount} ${category.label.toLowerCase()} variants`;
  for (const button of tabs.children) {
    button.setAttribute('aria-pressed', String(button.textContent === category.title));
  }
}

for (const category of categories) {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = category.title;
  button.addEventListener('click', () => render(category));
  tabs.append(button);
}
render(categories[0]);
