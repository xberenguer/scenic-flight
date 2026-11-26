const treeForms = ['Rounded', 'Pointed', 'Leaning', 'Forked', 'Spire', 'Umbrella', 'Vase', 'Fan', 'Cluster', 'Crescent', 'Tiered', 'Wind swept', 'Three lobed', 'Cut crown', 'Split ridge', 'Cushion'];
const treeFootprints = ['Compact', 'Wide', 'Deep', 'Broad'];
const treeSpecies = ['Oak', 'Spruce', 'Juniper', 'Maple', 'Spruce', 'Oak', 'Aspen', 'Aspen', 'Oak', 'Juniper', 'Spruce', 'Juniper', 'Maple', 'Maple', 'Aspen', 'Oak'];
const cactusSpecies = ['Saguaro', 'Organ pipe', 'Prickly pear', 'Golden barrel'];
const rockForms = ['Tumbled granite', 'Sandstone ledge', 'Basalt shards', 'Pale scree'];
const shrubForms = ['Sagebrush', 'Bunchgrass', 'Saltbush', 'Flowering yucca'];
const wetlandForms = ['Bulrush', 'Cattail', 'Iris', 'Willow scrub'];
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

const [wagonMetadata, vesselMetadata, serviceVesselMetadata] = await Promise.all(
  ['wagons', 'vessels', 'service-vessels'].map(name => fetch(`assets/artwork/${name}.json`)
    .then(response => response.ok ? response.json() : [])
    .catch(() => [])),
);
const serviceVesselFamilies = ['Harbor response', 'Marine research', 'Marine maintenance', 'Island and offshore supply', 'Fisheries support'];

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
const summitNames = await fetch('assets/artwork/summit-landmarks.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const summitFamilies = ['Lookouts', 'Shelters', 'Relay and weather stations', 'Stone landmarks', 'Observatories and surveying'];
const townNames = await fetch('assets/artwork/town-landmarks.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const townFamilies = ['Public services', 'Schools and recreation', 'Markets and depots', 'Utilities', 'Local industry'];
const industryNames = await fetch('assets/artwork/industrial-landmarks.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const industryFamilies = ['Utilities', 'Materials and foundries', 'Manufacturing and repair', 'Freight and distribution', 'Food and agricultural processing'];
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

const mountainNames = [
  'Creeping juniper', 'Stunted spruce', 'Mountain mahogany', 'Split pine snag', 'Uprooted pine',
  'Heather bank', 'Alpine avens', 'Dwarf willow', 'Bunchgrass ring', 'Gentian spikes',
  'Quartz seam', 'Slate blades', 'Rock step', 'Split granite', 'Basalt columns',
  'Snow cornice', 'Rim ice fin', 'Snow pocket', 'Wind crust', 'Rimed boulder',
  'Boulder train', 'Glacial pavement', 'Paired erratics', 'Moraine crest', 'Meltwater cobbles',
];
const mountainFamilies = ['Upper forest', 'Alpine meadow', 'Exposed rock', 'Snowfields', 'Scree and glacial deposits'];

const freshwaterNames = [
  'Cobble ridge', 'Sand ripples', 'Shale shingle', 'Flood wrack', 'Root wad',
  'River log', 'Forked driftwood', 'Silt butte', 'Pebble fan', 'Channel boulders',
  'Cattail stand', 'Bulrush umbels', 'Sweet flag', 'Bur-reed', 'Reed wall',
  'Marsh marigold', 'Iris stand', 'Meadowsweet', 'Loosestrife', 'Water mint',
  'Willow stool', 'Alder saplings', 'Red-osier thicket', 'Bracken break', 'Rush tussocks',
];
const freshwaterFamilies = ['Exposed bars', 'Exposed bars', 'Reedbeds', 'Wet-meadow flowers', 'Wet-meadow foliage'];

const streetBuildingNames = [
  'Cross-gable house', 'Staggered split-level house', 'Courtyard studio house', 'Hip-roof home with garage', 'Desert terrace house',
  'Stepped duplex', 'Offset gable duplex', 'Mansard duplex', 'Barrel-roof duplex', 'Opposed shed-roof duplex',
  'Veranda row', 'Terrace corner row', 'Drum-front row', 'Staggered gable row', 'Recessed walk-up flats',
  'Barrel shed', 'Monitor-roof factory', 'Distribution cross-dock', 'Kilnhouse', 'Lumber shed',
  'Corner pharmacy', 'Neighborhood cinema', 'Stall market', 'Street hotel', 'Bakery with service wing',
];
const streetBuildingFamilies = ['Detached homes', 'Duplexes', 'Terraced rows', 'Industrial', 'Main street'];

const airfieldBuildingNames = [
  'Gabled hangar with lean-to', 'Paired barrel hangars', 'Shed-roof hangar', 'Hipped hangar with workshop', 'Barrel hangar with raised skylight',
  'Monitor-roof hangar', 'Three-bay sawtooth hangar', 'Wide-span hangar with office wing', 'Split maintenance bays', 'Flat-roof hangar with clerestory',
  'Terminal with glazed canopy', 'Open-court terminal', 'Terminal with round concourse', 'Twin-vault terminal', 'Stepped regional concourse',
  'Capped control tower', 'Round tower and operations hall', 'Stepped operations tower', 'Radio tower compound', 'Fire and operations station',
  'Maintenance shed with side bay', 'Paired fuel tanks', 'Twin-vault storehouse', 'Service station with rear bay', 'Courtyard workshops',
];
const airfieldBuildingFamilies = ['Country hangars', 'Regional hangars', 'Terminals', 'Operations', 'Airport services'];

const woodlandNames = [
  'Spruce seedling', 'Hemlock whorls', 'Yew bush', 'Hazel coppice', 'Birch saplings',
  'Ostrich fern', 'Wood sedge', 'Broadleaf hellebore', 'Foamflower mat', 'Evergreen holly',
  'Nurse log', 'Split stump', 'Windthrow and roots', 'Bark heap', 'Branch tangle',
  'Shelf fungi', 'Parasol patch', 'Morel cluster', 'Puffball ring', 'Coral fungi',
  'Bracken fronds', 'Foxglove', 'Rose-hip bramble', 'Bilberry mound', 'Honeysuckle arch',
];
const woodlandFamilies = ['Young trees and shrubs', 'Shade plants', 'Fallen wood', 'Fungi', 'Woodland edges'];

const drylandNames = [
  'Piled wash cobbles', 'Dry root fan', 'Gravel ribs', 'Cutbank shelf', 'Drift branch',
  'Dune slip face', 'Ripple field', 'Sand grass line', 'Dune skirt', 'Sand verbena mat',
  'Sandstone pedestal', 'Twin sandstone fins', 'Banded sandstone', 'Basalt talus', 'Desert pavement',
  'Creosote ring', 'Saltbush hummocks', 'Brittlebush', 'Blackbrush', 'Mojave aster',
  'Spanish bayonet', 'Agave stalk', 'Cholla skeleton', 'Beavertail clump', 'Barrel cactus group',
];
const drylandFamilies = ['Dry washes', 'Dunes', 'Dryland rock', 'Scrub and flowers', 'Succulents'];

const grasslandNames = [
  'Oxeye daisies', 'Knapweed', 'Bird’s-foot trefoil', 'Chicory', 'Yarrow umbels',
  'Bluestem fan', 'Fescue tufts', 'Oat grass', 'Foxtail patch', 'Feather grass',
  'Hawthorn scrub', 'Gorse cushion', 'Broom stand', 'Dog rose', 'Juniper mound',
  'Limestone pavement', 'Chalk hummocks', 'Glacial boulder', 'Scattered flint', 'Low stone bank',
  'Wind-pruned thorn', 'Weathered log', 'Bracken island', 'Thorn root ball', 'Meadow anthill',
];
const grasslandFamilies = ['Wildflowers', 'Native grasses', 'Scrub', 'Meadow rocks', 'Meadow-floor details'];

const headlandNames = [
  'Sand hopper burrows', 'Razor shell fan', 'Kelp ribbon', 'Cork wrack', 'Barnacle cobbles',
  'Beach morning glory', 'Seaside daisy', 'Honckenya cushion', 'Dune spurge', 'Beach evening primrose',
  'Granite shingle', 'Flint spit', 'Sandstone ledge', 'Ironstone rubble', 'Chalk breccia',
  'Dune willow', 'Bayberry thicket', 'Sea buckthorn', 'Coastal heather', 'Salt-burned juniper',
  'Sand fescue', 'Lyme grass', 'Sea rush', 'Dune reed', 'Salt meadow hay',
];
const headlandFamilies = ['Strandline debris', 'Coastal flowers', 'Coastal stone', 'Wind-pruned scrub', 'Shore grasses'];

const alpineGroundNames = [
  'Wind-sculpted drift', 'Snow bridge lip', 'Snow pillows', 'Frosted talus', 'Snowbreak slabs',
  'Blue ice ribs', 'Pressure hummocks', 'Meltwater runnels', 'Refrozen lens', 'Layered ice step',
  'Garnet scree', 'Shale book', 'Quartzite needles', 'Frost-split tor', 'Veined schist',
  'Mixed erratics', 'Sorted stone ring', 'Ice-marginal bank', 'Glacial boulder fan', 'Moraine cobble ribs',
  'White saxifrage', 'Alpine poppy', 'Willow mat', 'Lichen cushions', 'Snowbed sedge',
];
const alpineGroundFamilies = ['Snow deposits', 'Glacier ice', 'Exposed rock', 'Glacial debris', 'Alpine plants'];
const floodplainNames = [
  'Peat hummocks', 'Sphagnum lobes', 'Fen marl ledge', 'Tussock islands', 'Bank overhang',
  'Ragged robin', 'Fen orchid', 'Grass of Parnassus', 'Yellow rattle', 'Globeflower',
  'Reed panicles', 'Spike-rush lawn', 'Flowering rush', 'Arrowhead stand', 'Reed cane litter',
  'Clay scrolls', 'Gravel tongue', 'Snag with silt apron', 'Iron seep crust', 'Sandbar ripple',
  'Bog myrtle', 'Leatherleaf mat', 'Dwarf willow', 'Buckthorn fen clump', 'Cranberry runner',
];
const floodplainFamilies = ['Wet ground', 'Meadow flowers', 'Emergent plants', 'Flood deposits', 'Wetland shrubs'];
const mesaNames = await fetch('assets/artwork/mesa-profiles.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const mesaFamilies = ['Broad caps', 'Incised rims', 'Terraces', 'Narrow divides', 'Pillars and fins'];
const countrysideNames = await fetch('assets/artwork/countryside-buildings.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const countrysideFamilies = ['Overlooks', 'Forest services', 'Trail access', 'Regional road services', 'Highway rest stops'];

const coastalRockNames = await fetch('assets/artwork/coastal-rocks.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const coastalRockFamilies = ['Sea stacks', 'Skerries and reefs', 'Sea arches'];

const lighthouseNames = await fetch('assets/artwork/lighthouses.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const lighthouseFamilies = ['Masonry', 'Keeper towers', 'Open-frame towers'];

const cablewayNames = await fetch('assets/artwork/cableways.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const cablewayFamilies = ['Timber', 'Brick', 'Stone', 'Aluminum', 'Enamel'];

const bridgeNames = await fetch('assets/artwork/bridges.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const riverIslandNames = await fetch('assets/artwork/river-islands.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const riverIslandFamilies = ['Reed', 'Gravel', 'Sand', 'Meadow', 'Stone'];

const glacierNames = await fetch('assets/artwork/glaciers.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const glacierFamilies = ['Snow basins', 'Crevasse fields', 'Ice termini', 'Moraine streaks', 'Mixed forms'];

const tarnNames = await fetch('assets/artwork/tarns.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const tarnFamilies = ['Sedge and reeds', 'Sand shelves', 'Gravel shores', 'Rock and moraine', 'Alpine meadows'];

const moraineNames = await fetch('assets/artwork/moraines.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const moraineFamilies = ['Terminal arcs', 'Asymmetric banks', 'Recessional ridges', 'Debris aprons', 'Scalloped ridges'];

const waterTowerNames = await fetch('assets/artwork/water-towers.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);
const waterTowerFamilies = ['Pedestal', 'Steel frame', 'Masonry', 'Twin pier', 'Portal'];

const solarLayoutNames = await fetch('assets/artwork/solar-layouts.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const alluvialNames = await fetch('assets/artwork/alluvial-fans.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const headwallNames = await fetch('assets/artwork/columnar-headwalls.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const cultivatedNames = await fetch('assets/artwork/cultivated-fields.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const coveDuneNames = await fetch('assets/artwork/cove-dunes.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const quayWorkshopNames = await fetch('assets/artwork/quay-workshops.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const quarryNames = await fetch('assets/artwork/quarry-profiles.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const apronNames = await fetch('assets/artwork/airfield-aprons.txt')
  .then(response => response.ok ? response.text() : '')
  .then(text => text.trim().split('\n'))
  .catch(() => []);

const categories = [
  { title: 'Airfield aprons', label: 'Apron layout', atlas: 'airfield-aprons.png',
    columns: 5, rows: 5, name: index => apronNames[index], animation: 'airfield-aprons.gif' },
  { title: 'Hillside quarries', label: 'Quarry', atlas: 'quarry-profiles.png',
    columns: 5, rows: 5, name: index => quarryNames[index], animation: 'quarry-profiles.gif' },
  { title: 'Quayside workshops', label: 'Waterfront building', atlas: 'quay-workshops.png',
    columns: 5, rows: 5, name: index => quayWorkshopNames[index], animation: 'quay-workshops.gif' },
  { title: 'Sheltered cove dunes', label: 'Cove dune', atlas: 'cove-dunes.png',
    columns: 5, rows: 5, name: index => coveDuneNames[index], animation: 'cove-dunes.gif' },
  { title: 'Cultivated fields', label: 'Field layout', atlas: 'cultivated-fields.png',
    columns: 5, rows: 5, name: index => cultivatedNames[index], animation: 'cultivated-fields.gif' },
  { title: 'Columnar rock faces', label: 'Rock face', atlas: 'columnar-headwalls.png',
    columns: 5, rows: 5, name: index => headwallNames[index], animation: 'columnar-headwalls.gif' },
  { title: 'Desert alluvial fans', label: 'Alluvial fan', atlas: 'alluvial-fans.png',
    columns: 5, rows: 5, name: index => alluvialNames[index], animation: 'alluvial-fans.gif' },
  { title: 'Solar array layouts', label: 'Solar array', atlas: 'solar-layouts.png',
    columns: 5, rows: 5, name: index => solarLayoutNames[index], animation: 'solar-layouts.gif' },
  { title: 'Municipal water towers', label: 'Water tower', atlas: 'water-towers.png',
    columns: 5, rows: 5, name: index => waterTowerNames[index],
    family: index => waterTowerFamilies[Math.floor(index / 5)], animation: 'water-towers.gif' },
  { title: 'Glacial moraines', label: 'Moraine', atlas: 'moraines.png',
    columns: 5, rows: 5, name: index => moraineNames[index],
    family: index => moraineFamilies[Math.floor(index / 5)], animation: 'moraines.gif' },
  { title: 'Mountain lake shores', label: 'Lake shore', atlas: 'tarns.png',
    columns: 5, rows: 5, name: index => tarnNames[index],
    family: index => tarnFamilies[Math.floor(index / 5)], animation: 'tarns.gif' },
  { title: 'Alpine glaciers', label: 'Glacier', atlas: 'glaciers.png',
    columns: 5, rows: 5, name: index => glacierNames[index],
    family: index => glacierFamilies[Math.floor(index / 5)], animation: 'glaciers.gif' },
  { title: 'River islands', label: 'River island', atlas: 'river-islands.png',
    columns: 5, rows: 5, name: index => riverIslandNames[index],
    family: index => riverIslandFamilies[Math.floor(index / 5)], animation: 'river-islands.gif' },
  { title: 'River and coastal bridges', label: 'Bridge', atlas: 'bridges.png',
    columns: 5, rows: 5, name: index => bridgeNames[index],
    family: index => index < 10 ? 'Low girder and parapet' : 'Open truss and arch', animation: 'bridges.gif' },
  { title: 'Mountain cableways', label: 'Cableway', atlas: 'cableways.png',
    columns: 5, rows: 5, name: index => cablewayNames[index],
    family: index => cablewayFamilies[Math.floor(index / 5)], animation: 'cableways.gif' },
  { title: 'Lighthouses', label: 'Lighthouse', atlas: 'lighthouses.png',
    columns: 5, rows: 5, name: index => lighthouseNames[index],
    family: index => lighthouseFamilies[index < 9 ? 0 : index < 17 ? 1 : 2], animation: 'lighthouses.gif' },
  { title: 'Coastal rock formations', label: 'Coastal rock formation', atlas: 'coastal-rocks.png',
    columns: 5, rows: 5, name: index => coastalRockNames[index],
    family: index => coastalRockFamilies[index < 9 ? 0 : index < 17 ? 1 : 2], animation: 'coastal-rocks.gif' },
  { title: 'Countryside buildings', label: 'Countryside building', atlas: 'countryside-buildings.png',
    columns: 5, rows: 5, name: index => countrysideNames[index],
    family: index => countrysideFamilies[Math.floor(index / 5)], animation: 'countryside-buildings.gif' },
  { title: 'Sandstone mesas', label: 'Sandstone mesa', atlas: 'mesa-profiles.png',
    columns: 5, rows: 5, name: index => mesaNames[index],
    family: index => mesaFamilies[Math.floor(index / 5)], animation: 'mesa-profiles.gif' },
  { title: 'Floodplain details', label: 'Floodplain detail', atlas: 'floodplain-details.png',
    columns: 5, rows: 5, name: index => floodplainNames[index],
    family: index => floodplainFamilies[Math.floor(index / 5)], animation: 'floodplain-details.gif' },
  { title: 'Industrial landmarks', label: 'Industrial landmark', atlas: 'industrial-landmarks.png',
    columns: 5, rows: 5, name: index => industryNames[index],
    family: index => industryFamilies[Math.floor(index / 5)], animation: 'industrial-landmarks.gif' },
  { title: 'Alpine ground details', label: 'Alpine ground detail', atlas: 'alpine-ground-details.png',
    columns: 5, rows: 5, name: index => alpineGroundNames[index],
    family: index => alpineGroundFamilies[Math.floor(index / 5)], animation: 'alpine-ground-details.gif' },
  { title: 'Headland details', label: 'Headland detail', atlas: 'headland-details.png', columns: 5, rows: 5,
    name: index => headlandNames[index], family: index => headlandFamilies[Math.floor(index / 5)],
    animation: 'headland-details.gif' },
  { title: 'Grassland details', label: 'Grassland detail', atlas: 'grassland-details.png', columns: 5, rows: 5,
    name: index => grasslandNames[index], family: index => grasslandFamilies[Math.floor(index / 5)],
    animation: 'grassland-details.gif' },
  { title: 'Dryland details', label: 'Dryland detail', atlas: 'dryland-details.png', columns: 5, rows: 5,
    name: index => drylandNames[index], family: index => drylandFamilies[Math.floor(index / 5)],
    animation: 'dryland-details.gif' },
  { title: 'Woodland details', label: 'Woodland detail', atlas: 'woodland-details.png', columns: 5, rows: 5,
    name: index => woodlandNames[index], family: index => woodlandFamilies[Math.floor(index / 5)],
    animation: 'woodland-details.gif' },
  { title: 'Airfield buildings', label: 'Airfield building', atlas: 'airfield-buildings.png', columns: 5, rows: 5,
    name: index => airfieldBuildingNames[index], family: index => airfieldBuildingFamilies[Math.floor(index / 5)],
    animation: 'airfield-buildings.gif' },
  { title: 'Street buildings', label: 'Street building', atlas: 'street-buildings.png', columns: 5, rows: 5,
    name: index => streetBuildingNames[index], family: index => streetBuildingFamilies[Math.floor(index / 5)],
    animation: 'street-buildings.gif' },
  { title: 'Freshwater details', label: 'Freshwater detail', atlas: 'freshwater-details.png', columns: 5, rows: 5,
    name: index => freshwaterNames[index], family: index => freshwaterFamilies[Math.floor(index / 5)],
    animation: 'freshwater-details.gif' },
  { title: 'Mountain details', label: 'Mountain detail', atlas: 'mountain-details.png', columns: 5, rows: 5,
    name: index => mountainNames[index], family: index => mountainFamilies[Math.floor(index / 5)],
    animation: 'mountain-details.gif' },
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
  { title: 'Skyline landmarks', label: 'Skyline landmark', atlas: 'skyline-landmarks.png', columns: 10, rows: 8, count: skylineNames.length,
    name: index => skylineNames[index], animation: 'skyline-landmarks.gif' },
  { title: 'Summit landmarks', label: 'Summit landmark', atlas: 'summit-landmarks.png', columns: 5, rows: 5,
    name: index => summitNames[index], family: index => summitFamilies[Math.floor(index / 5)],
    animation: 'summit-landmarks.gif' },
  { title: 'Town landmarks', label: 'Town landmark', atlas: 'town-landmarks.png', columns: 5, rows: 5,
    name: index => townNames[index], family: index => townFamilies[Math.floor(index / 5)],
    animation: 'town-landmarks.gif' },
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
  { title: 'Service vessels', label: 'Service vessel', atlas: 'service-vessels.png', columns: 5, rows: 5,
    name: index => serviceVesselMetadata[index]?.configuration,
    family: index => serviceVesselFamilies[Math.floor(index / 5)], animation: 'service-vessels.gif' },
  { title: 'Vessels', label: 'Vessel', atlas: 'vessels.png', columns: 10, rows: 25,
    family: index => shipTypes[Math.floor(index / 50)][0],
    detail: index => [vesselMetadata[index]?.configuration, shipTypes[Math.floor(index / 50)][1]]
      .filter(Boolean).join(' · ') },
  { title: 'Wetland plants', label: 'Wetland plant', atlas: 'wetlands.png',
    family: index => wetlandForms[index >> 6], traits: ['Width', 'Height', 'Spread'] },
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
