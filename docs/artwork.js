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
const waterfrontLandmarks = [
  ['Boat shed', 'Gabled shed and timber landing'],
  ['Picnic pavilion', 'Clipped-corner shelter beside the boardwalk'],
  ['Waterfront lookout', 'Raised viewing tower above a low deck'],
  ['Waterside market', 'Gabled hall with striped awnings'],
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

const towerFamilies = [
  'Office slab', 'Art Deco shoulder', 'Rear terraces', 'Pencil tower', 'Grid block',
  'Offset penthouse', 'Thin end wall', 'Broad hotel', 'Stacked terraces', 'Edge spire',
  'Compact stack', 'End-set crown', 'Portal block', 'Library terraces', 'Observatory',
  'Diagonal slabs', 'Setback warehouse', 'Corner block', 'Twin setbacks', 'Tall glass',
  'Stone shaft', 'Brick stair tower', 'Side-mounted crown', 'Narrow plan', 'Civic podium',
];

const skylineNames = await fetch('assets/artwork/skyline-landmarks.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const habitatNames = [
  'Ocotillo', 'Yucca rosette', 'Joshua tree', 'Honeycomb sandstone', 'Varnished slabs',
  'Lupine spires', 'Poppy patch', 'Mullein stalks', 'Rabbitbrush', 'Thistle crown',
  'Fallen trunk', 'Uprooted snag', 'Fungus stump', 'Sword fern', 'Bramble arch',
  'Granite tor', 'Lichen boulder', 'Scree fan', 'Cushion plant', 'Moraine erratic',
  'Driftwood', 'Wrack windrow', 'Sea thrift', 'Marram grass', 'Shingle berm',
  'Bog cotton', 'Skunk cabbage', 'Tussock sedge', 'Horsetail', 'Mud hummock',
];
const habitatFamilies = ['Desert', 'Meadow', 'Woodland', 'Alpine', 'Coast', 'Wetland'];
const buildingRows = [
  ['Deep-eave chalet', 'Adobe patio house', 'Pyramidal cottage', 'A-frame cabin', 'Courtyard ranch', 'Gambrel conversion'],
  ['Butterfly duplex', 'Linked villas', 'Twin saltboxes', 'Patio duplex', 'Barrel-roof homes'],
  ['Vaulted townhouses', 'Stairhouse row', 'Stepped-gable row', 'Sawtooth row', 'Hillside terraces'],
  ['Bulk silos', 'Power house', 'Twin-bay hangar', 'Cooling plant', 'Repair works', 'Glasshouse range'],
  ['Market arcade', 'Roadside diner', 'Corner bank', 'Theater', 'Corner storefronts'],
];
const buildingTiles = buildingRows.flatMap((row, y) => row.map((name, x) => ({ name, x, y })));
const buildingFamilies = ['Detached homes', 'Duplexes', 'Townhouses', 'Industry', 'Main street'];

const coastalNames = [
  'Shell bank', 'Pumice raft', 'Kelp stipes', 'Tangled roots', 'Oyster ridge',
  'Ice plant', 'Dune sedge', 'Sand verbena', 'Beach pea', 'Salt-pruned pine',
  'Pebble terrace', 'Boulder tripod', 'Basalt fingers', 'Split slab', 'Eroded sandstone',
  'Hauled skiff', 'Lobster pots', 'Net rack', 'Fish crates', 'Oar trestle',
  'Rescue stand', 'Dune gate', 'Shade sail', 'Beach steps', 'Trail kiosk',
];
const coastalGroups = ['Strand debris', 'Dune plants', 'Rock formations', 'Working beaches', 'Beach access'];

const farmNames = [
  'Round bales', 'Straw stack', 'Covered wagon', 'Grain auger', 'Combine',
  'Gambrel barn', 'Pole shed', 'Milk parlor', 'Glasshouse', 'Root cellar',
  'Tank stand', 'Pump cabin', 'Hose reel', 'Stone trough', 'Sluice house',
  'Orchard bins', 'Vine trellis', 'Picking ladder', 'Picking platform', 'Cold store',
  'Produce stall', 'Weighbridge', 'Feed depot', 'Windpump', 'Field gate',
];
const farmGroups = ['Harvest', 'Farm buildings', 'Water and irrigation', 'Orchards', 'Farm access'];

const railwayNames = [
  'Station clock', 'Timetable case', 'Platform marker', 'Lamp pair', 'Dispatch hut',
  'Luggage shelter', 'Parcel trolley', 'Ticket kiosk', 'Waiting screen', 'Newsstand',
  'Semaphore', 'Signal cabinet', 'Tool locker', 'Sand box', 'Track-gauge rack',
  'Jib crane', 'Loading hoist', 'Weigh desk', 'Sack hopper', 'Hose manifold',
  'Timber bunks', 'Coil cradles', 'Milk churns', 'Drum pallet', 'Stacked crates',
];
const railwayFamilies = ['Platform equipment', 'Passenger services', 'Maintenance', 'Freight loading', 'Freight storage'];

const harborNames = [
  'Sail loft', 'Yacht club', 'Boatworks', 'Marina office', 'Chandlery',
  'Glazed concourse', 'Customs hall', 'Transfer station', 'Ticket wings', 'Ferry gallery',
  'Sawtooth warehouse', 'Refrigerated hall', 'Bonded depot', 'Transfer hangar', 'Mill supply depot',
  'Dinghy rack', 'Boat lift', 'Fuel island', 'Mast cradle', 'Floating-dock staging',
  'Ramp winch', 'Baggage-cart shed', 'Shuttle canopy', 'Ferry information pylon', 'Service stairs',
  'Weigh station', 'Pipe racks', 'Transload hopper', 'Pump compound', 'Forklift shelter',
];
const harborFamilies = [
  'Marina buildings', 'Ferry terminals', 'Freight warehouses',
  'Marina equipment', 'Ferry equipment', 'Freight equipment',
];

const roadsideNames = [
  'Viewfinders', 'Relief map', 'Viewing deck', 'Stone monument', 'Lookout steps',
  'Panorama canopy', 'Sundial', 'Wind screen', 'Cairn pair', 'Orientation wheel',
  'Trail map', 'Gear rack', 'Cycle shelter', 'Boot bench', 'Trail arch',
  'Emergency call box', 'Bottle fountain', 'Picnic lean-to', 'Trail register', 'Fire cache',
  'Fuel canopy', 'Pump house', 'Roadside carport', 'Charging bank', 'Route kiosk',
  'Produce market', 'Picnic pavilion', 'Vending shelter', 'Bus shelter', 'Road gate',
];
const roadsideFamilies = ['Scenic overlooks', 'Trailheads', 'Rest stops'];

const categories = [
  { title: 'Roadside models', label: 'Roadside model', atlas: 'roadside-models.png', columns: 5, rows: 6,
    name: index => roadsideNames[index], family: index => roadsideFamilies[Math.floor(index / 10)],
    animation: 'roadside-models.gif' },
  { title: 'Harbor models', label: 'Harbor model', atlas: 'harbor-models.png', columns: 5, rows: 6,
    name: index => harborNames[index], family: index => harborFamilies[Math.floor(index / 5)],
    animation: 'harbor-models.gif' },
  { title: 'Railway fixtures', label: 'Railway fixture', atlas: 'rail-stations.png', columns: 5, rows: 5,
    name: index => railwayNames[index], family: index => railwayFamilies[Math.floor(index / 5)],
    animation: 'rail-stations.gif' },
  { title: 'Farm details', label: 'Farm detail', atlas: 'farm-details.png', columns: 5, rows: 5,
    name: index => farmNames[index], family: index => farmGroups[Math.floor(index / 5)],
    animation: 'farm-details.gif' },
  { title: 'Coastal details', label: 'Coastal detail', atlas: 'coastal-details.png', columns: 5, rows: 5,
    name: index => coastalNames[index], family: index => coastalGroups[Math.floor(index / 5)],
    animation: 'coastal-details.gif' },
  { title: 'Skyline landmarks', label: 'Skyline landmark', atlas: 'skyline-landmarks.png', columns: 5, rows: 5,
    name: index => skylineNames[index], animation: 'skyline-landmarks.gif' },
  { title: 'Neighborhood buildings', label: 'Neighborhood building', atlas: 'neighborhood-buildings.png',
    columns: 6, rows: 5, count: buildingTiles.length,
    tile: index => [buildingTiles[index].x, buildingTiles[index].y],
    name: index => buildingTiles[index].name, family: index => buildingFamilies[buildingTiles[index].y] },
  { title: 'Habitat details', label: 'Habitat detail', atlas: 'habitat-details.png', columns: 5, rows: 6,
    name: index => habitatNames[index], family: index => habitatFamilies[Math.floor(index / 5)] },
  { title: 'City towers', label: 'Tower', atlas: 'towers.png', columns: 10, rows: 25,
    family: index => towerFamilies[Math.floor(index / 10)] },
  { title: 'Trees', label: 'Tree', atlas: 'trees.png', family: index => treeSpecies[index & 15] },
  { title: 'Rocks', label: 'Rock', atlas: 'rocks.png', family: index => rockForms[index >> 6], traits: ['Width', 'Depth', 'Relief'] },
  { title: 'Bushes and grasses', label: 'Shrub', atlas: 'bushes.png', family: index => shrubForms[index >> 6], traits: ['Width', 'Height', 'Spread'] },
  { title: 'Cacti', label: 'Cactus', atlas: 'cacti.png', family: index => cactusSpecies[index >> 6],
    traits: ['Girth', 'Height', 'Spread'] },
  { title: 'Freight wagons', label: 'Freight wagon', atlas: 'wagons.png', columns: 10, rows: 25,
    family: index => wagonMetadata[index]?.family,
    name: index => wagonMetadata[index]?.equipment,
    reference: index => wagonMetadata[index]?.reference,
    detail: index => {
      const wagon = wagonMetadata[index];
      if (!wagon) return '';
      return [wagon.cargo, wagon.purpose || wagon.assignment].filter(Boolean).join('. ');
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
  { title: 'Waterfront landmarks', label: 'Landmark', atlas: 'waterfront-landmarks.png', columns: 4, rows: 1,
    name: index => waterfrontLandmarks[index][0], family: index => waterfrontLandmarks[index][0],
    detail: index => waterfrontLandmarks[index][1] },
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
  return category.count ?? (category.columns || 16) * (category.rows || 16);
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
    const title = category.name?.(index) || `${family || category.label} ${String(index + 1).padStart(3, '0')}`;
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
      const reference = category.reference?.(index);
      if (reference?.startsWith('https://')) {
        const link = document.createElement('a');
        link.href = reference;
        link.target = '_blank';
        link.rel = 'noopener';
        link.textContent = 'Equipment reference';
        viewerInfo.append(' ', link);
      }
      viewerInfo.hidden = !viewerInfo.textContent;
      const animation = category.animation && !matchMedia('(prefers-reduced-motion: reduce)').matches;
      viewerMedia.replaceChildren(sprite(animation ? { ...category, atlas: category.animation } : category, index));
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
