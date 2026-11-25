const categories = [
  { title: 'Trees', label: 'Tree', atlas: 'trees.png', traits: ['Crown form', 'Width', 'Depth', 'Height'] },
  { title: 'Rocks', label: 'Rock', atlas: 'rocks.png', traits: ['Width', 'Depth', 'Relief', 'Crown'] },
  { title: 'Bushes', label: 'Bush', atlas: 'bushes.png', traits: ['Width', 'Height', 'Spread', 'Crown'] },
  { title: 'Cacti', label: 'Cactus', atlas: 'cacti.png', traits: ['Girth', 'Height', 'Arm reach', 'Arm arrangement'] },
];
const atlasColumns = 16;
const variantCount = 256;
const tabs = document.querySelector('#artwork-categories');
const grid = document.querySelector('#artwork-grid');
const count = document.querySelector('#artwork-count');
const viewer = document.querySelector('#viewer');
const viewerMedia = document.querySelector('#viewer-media');
const viewerTitle = document.querySelector('#viewer-title');
const viewerInfo = document.querySelector('#viewer-info');

function sprite(atlas, index) {
  const image = document.createElement('span');
  const x = index % atlasColumns;
  const y = Math.floor(index / atlasColumns);
  image.className = 'artwork-sprite';
  image.style.backgroundImage = `url("assets/artwork/${atlas}")`;
  image.style.backgroundPosition = `${x * 100 / (atlasColumns - 1)}% ${y * 100 / (atlasColumns - 1)}%`;
  image.setAttribute('aria-hidden', 'true');
  return image;
}

function render(category) {
  const entries = document.createDocumentFragment();
  for (let index = 0; index < variantCount; index++) {
    const entry = document.createElement('li');
    const button = document.createElement('button');
    const number = String(index).padStart(3, '0');
    const title = `${category.label} ${number}`;
    button.type = 'button';
    button.className = 'artwork-tile';
    button.setAttribute('aria-label', title);
    const caption = document.createElement('span');
    caption.className = 'artwork-number';
    caption.textContent = number;
    button.append(sprite(category.atlas, index), caption);
    button.addEventListener('click', () => {
      viewerTitle.textContent = title;
      viewerInfo.textContent = category.traits.map((trait, axis) => {
        const form = (index >> (axis * 2)) & 3;
        if (category.label === 'Tree' && axis === 0) return ['Rounded', 'Pointed', 'Leaning', 'Forked'][form];
        return `${trait} ${form + 1} of 4`;
      }).join(' · ');
      viewerInfo.hidden = false;
      viewerMedia.replaceChildren(sprite(category.atlas, index));
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
