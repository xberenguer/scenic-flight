// The game uses compressed interpretations of real loading practices and equipment.
// Each entry describes a distinct body, operating configuration, or visible load.
struct Profile {
    name: &'static str,
    purpose: &'static str,
}

const fn profile(name: &'static str, purpose: &'static str) -> Profile {
    Profile { name, purpose }
}

const PROFILES: [[[Profile; 5]; 5]; 10] = [
    // Intermodal: TTX articulated 40-foot and 53-foot well equipment.
    [
        [
            profile("Short recessed empty well", "Return an empty maritime well after a terminal delivers its standard container."),
            profile("Hitch-equipped empty well", "Reposition an unloaded dual-use frame for a later trailer or container movement."),
            profile("Long domestic empty well", "Send a longer unloaded domestic well to the next inland loading ramp."),
            profile("Open crossbearer empty well", "Bring back a lightweight well with the load-bearing crossmembers exposed."),
            profile("Deep-sill empty well", "Return a reinforced well to the port after a heavy container assignment."),
        ],
        [
            profile("Short well with an end-loaded 20", "Use the end locks to carry one short import box at the edge of a maritime well."),
            profile("Center-set single 20", "Balance one short import load nearer the center of a standard frame for inland transfer."),
            profile("Long well with a single pallet box", "Move one short container in a long domestic well between depot equipment assignments."),
            profile("Open-frame single 20", "Carry a short container above the exposed crossbearers of a lighter maritime frame."),
            profile("Reinforced short-container well", "Accept a dense short container on a deep-sill frame for heavy dock freight."),
        ],
        [
            profile("Compact paired-20 well", "Use both short container positions to bring two import boxes inland in one well."),
            profile("Standard paired-20 well", "Carry two short loads in separate locked positions for sequential ground handling."),
            profile("Long mixed-height pair", "Accept two short boxes on the extended frame when the terminal has a mixed import queue."),
            profile("Crossbearer twin-box well", "Support the neighboring container ends at the middle of a lightweight open frame."),
            profile("Heavy-base two-box well", "Give dense paired imports a deeper load-bearing side sill on a port service."),
        ],
        [
            profile("Compact domestic single-stack", "Carry one shorter domestic container on a regional truck-to-rail service."),
            profile("Standard domestic single-stack", "Carry a single domestic box on a corridor without clearance for a second tier."),
            profile("Extended domestic single-stack", "Accommodate a long domestic box coming off a tractor at the intermodal terminal."),
            profile("Open-frame domestic single", "Use open crossmembers to keep tare down under one domestic box."),
            profile("Deep-sill domestic single", "Support a heavier long-distance domestic box without requiring the second level."),
        ],
        [
            profile("Low-well maritime double stack", "Seat the lower box deep in the well to keep two maritime containers within clearance."),
            profile("Standard maritime double stack", "Shift two levels of import boxes between port and inland ramps."),
            profile("Extended domestic double stack", "Pair two container tiers on a longer frame serving a cleared domestic corridor."),
            profile("Open-frame double stack", "Support a two-tier load on crossbearers with an open lightweight bottom structure."),
            profile("Deep-sill terminal double stack", "Reinforce the lower box support for a heavy container pair on a cleared main line."),
        ],
    ],
    // Dry enclosed freight: BNSF boxcar roles and Greenbrier's door/body options.
    [
        [
            profile("Paper roll slider", "Feed a printer through a centered entry suited to individual roll-clamp handling."),
            profile("Wide-opening roll car", "Make room for wide roll stock and clamp-truck positioning at a paper converter."),
            profile("Compact sealed paper car", "Use a snug plug-door body to protect a small order of moisture-sensitive rolls."),
            profile("Offset-door paper car", "Align the opening with a converter's loading bay and preserve longer floor space on one side."),
            profile("High-cube paper car", "Use the higher roof for tall rolls traveling from mill to converting warehouse."),
        ],
        [
            profile("Merchandise slider", "Use a standard forklift opening for mixed warehouse pallets going to a small depot."),
            profile("Wide double-slider merchandise car", "Load pallets on several forks through a wide warehouse door opening."),
            profile("Short sealed merchandise car", "Keep a smaller load of boxed consumer items sheltered through local switching."),
            profile("Offset-door merchandise car", "Set the entry off center to work at an asymmetrically placed warehouse dock."),
            profile("High-cube warehouse boxcar", "Use the taller interior for lighter, higher stacked merchandise pallets."),
        ],
        [
            profile("Beverage single-slider", "Deliver cases to a wholesaler with enough door width for standard beverage pallets."),
            profile("Full-width beverage entry", "Use a double opening for side-loading tall pallet stacks with a powered forklift."),
            profile("Compact gasketed beverage car", "Protect a smaller shipment of packaged drinks from dirty or wet loading conditions."),
            profile("Offset-plug beverage car", "Retain a sealed door for dusty routes and leave a longer uninterrupted pallet row."),
            profile("High-cube beverage car", "Use a tall body for lighter packaged drinks with more volume than floor weight."),
        ],
        [
            profile("Bagged-goods single-slider", "Deliver sealed ingredient sacks from a bagging plant to a nearby food producer."),
            profile("Bagged-goods double-slider", "Move wide bagged ingredient pallets quickly through a broad warehouse opening."),
            profile("Short gasketed ingredient car", "Use a sealed short body for small lots of moisture-sensitive bagged goods."),
            profile("Offset-door ingredient van", "Keep sacks dry on a covered route with a receiving door toward one end."),
            profile("Tall dry-ingredient boxcar", "Use greater cube for light dry ingredients shipped in protected pallet stacks."),
        ],
        [
            profile("Covered lumber slider", "Protect finished lumber boards and trim from rain between mill and dealer."),
            profile("Wide-door finished lumber car", "Clear long bundled boards through a wider side entry at the lumber warehouse."),
            profile("Compact sealed trim car", "Keep short moisture-sensitive millwork indoors through a low-volume shipment."),
            profile("Offset-door millwork car", "Leave enough uninterrupted floor for long moisture-sensitive packs beside the entry."),
            profile("High-cube dry lumber car", "Use a tall enclosed body for bulky finished lumber when weather protection matters."),
        ],
    ],
    // Food-grade gravity, dense mineral and high-volume pellet covered hoppers.
    [
        [
            profile("Compact two-bay grain hopper", "Take a small gravity-unloaded grain consignment out of a rural elevator."),
            profile("Ribbed three-bay grain hopper", "Stiffen a farm-service body carrying a larger grain load to a mill."),
            profile("Centerflow grain hopper", "Send several gravity outlets over a receiving pit one bay at a time."),
            profile("Deep-body grain hopper", "Give a stable long-grain flow to an elevator distributor with low receiving clearance."),
            profile("Extended grain hopper", "Use the longer fill trough and extra grain volume for export terminal runs."),
        ],
        [
            profile("Short twin-outlet cement hopper", "Reach the weight limit with a small volume of dense powdered cement."),
            profile("Ribbed cement hopper", "Carry heavy dry cement to a ready-mix depot in a reinforced short body."),
            profile("Compact center-discharge cement hopper", "Direct dense powder under the centerline into a covered receiving trench."),
            profile("Steep-gate cement hopper", "Use deep sloped bays to feed dry powder toward the unloading gate."),
            profile("Extended cement hopper", "Use a longer compact body for large construction batches without a grain-size shell."),
        ],
        [
            profile("Short clean-compartment resin hopper", "Carry a small batch of polymer pellets in a dedicated clean car."),
            profile("Three-bay resin hopper", "Keep longer batches of low-density pellets dry and release them in sections."),
            profile("High-volume centerflow resin hopper", "Use a smooth tapered section for clean transfer of plastic pellets."),
            profile("Deep-discharge resin hopper", "Concentrate light pellets at the outlet for a plant's unloading system."),
            profile("Extended large-cube pellet hopper", "Reach the needed pellet volume before reaching the route's load limit."),
        ],
        [
            profile("Short two-bay sand hopper", "Keep dense kiln-dried sand dry in a low-volume body on a short spur."),
            profile("Ribbed industrial sand hopper", "Support a heavy dry sand load on a compact ribbed gravity frame."),
            profile("Centered glass-sand hopper", "Discharge high-grade sand into a narrow factory receiving trench."),
            profile("Deep-gate industrial sand hopper", "Use steeper bay walls for a dry sand receiver that unloads by gravity."),
            profile("Extended dry-sand hopper", "Deliver a larger dry silica shipment in a low-profile mineral body."),
        ],
        [
            profile("Compact clean sugar hopper", "Keep a small load of bulk sugar sealed between a processor and food plant."),
            profile("Ribbed sugar hopper", "Carry a larger insulated-from-weather bulk sugar shipment in a reinforced shell."),
            profile("Centerflow sugar hopper", "Send food-grade sugar to a clean terminal with several underneath discharge points."),
            profile("Deep-bay sugar hopper", "Give bulk crystals a steeper path to their outlet in a gravity-equipped food terminal."),
            profile("Extended food-grade sugar hopper", "Send a full plant batch in a hopper dedicated to food-grade sugar."),
        ],
    ],
    // Greenbrier general service, asphalt, ethanol and high-pressure tank examples.
    [
        [
            profile("Small food-oil tank", "Move a small edible oil lot on a short car dedicated to sanitary ingredients."),
            profile("Standard food-oil tank", "Keep edible oil isolated from industrial liquids on the way to a processing plant."),
            profile("Long food-oil tank", "Carry higher volumes of low-density edible oil between larger food terminals."),
            profile("Jacketed food-oil tank", "Retain warmth so thicker food oil flows at the receiving connection."),
            profile("High-volume food-oil tank", "Use a larger food-service vessel for a full edible-oil delivery run."),
        ],
        [
            profile("Short heated asphalt tank", "Carry dense asphalt to a small paving depot with heat at unloading."),
            profile("Standard asphalt service tank", "Receive molten paving binder at a refinery and keep it in a short thermal vessel."),
            profile("Extended asphalt tank", "Carry an increased binder load while keeping space for heating hardware."),
            profile("Insulated asphalt tank", "Use a thermal jacket to keep paving binder warm between terminals."),
            profile("Large heated asphalt tank", "Supply a larger paving job with a compact, heat-retaining vessel."),
        ],
        [
            profile("Short ethanol service tank", "Move a smaller distillery shipment in a protected fuel-service vessel."),
            profile("Standard ethanol tank", "Carry ethanol from a blending source in a conventional fuel consist."),
            profile("Long ethanol tank", "Use additional capacity for a low-density ethanol refinery shipment."),
            profile("Thermally shielded ethanol tank", "Add a protective jacket for this flammable-liquid service."),
            profile("High-volume fuel ethanol tank", "Load a full-volume ethanol order at a dedicated fuel terminal."),
        ],
        [
            profile("Compact LPG pressure vessel", "Serve a short gas-terminal siding with an enclosed pressure-valve housing."),
            profile("Standard LPG pressure vessel", "Take liquefied gas to a distributor with its top valves under a bonnet."),
            profile("Long LPG pressure vessel", "Carry more liquefied propane without using an open liquid-tank fitting."),
            profile("Jacketed LPG pressure vessel", "Use a thermally protected pressure shell for a gas distribution shipment."),
            profile("Large LPG pressure vessel", "Use a tall rounded pressure shell and enclosed top valves for full gas loads."),
        ],
        [
            profile("Compact utility-water tank", "Serve an industrial water user on a short water-transfer spur."),
            profile("Standard industrial water tank", "Supply a utility customer in a water-only general-service tank."),
            profile("Long water transfer tank", "Carry more process water to a large industrial customer by a direct train."),
            profile("Jacketed utility-water tank", "Reduce freezing risk on a cold-weather industrial water transfer."),
            profile("Large process-water tank", "Use a higher-volume general-service vessel for an industrial bulk-water delivery."),
        ],
    ],
    // TTX and Union Pacific open flatcar freight and Greenbrier machinery analogs.
    [
        [
            profile("Short low-deck bulkhead return", "Return a low empty deck to a yard after dropping off its oversized load."),
            profile("Tall-bulkhead empty flat", "Return the built-in load stops to a steel or timber loading yard."),
            profile("Raised-end empty flat", "Send an empty frame with raised end supports back to the next pipe load."),
            profile("Depressed-deck empty return", "Reposition a low-centered machinery deck after it clears a movement."),
            profile("Reinforced open-deck return", "Bring a longer strengthened flat back to a fabrication terminal."),
        ],
        [
            profile("Twin pipe low flat", "Carry two utility pipes low on a deck to a nearby construction depot."),
            profile("Three-pipe bulkhead flat", "Stand three supported pipes between end bulkheads for longer shipment."),
            profile("Twin-pipe riser deck", "Elevate the long tubes on their support points for unloading access."),
            profile("Three-pipe depressed deck", "Use a lower deck for a wider-diameter pipe bundle along a tighter route."),
            profile("Extended heavy-pipe flat", "Send long large pipe on a reinforced frame to a distant utility yard."),
        ],
        [
            profile("Tracked excavator on low flat", "Use a low open deck to carry the excavator's high boom below the route limit."),
            profile("Wheeled loader on flatcar", "Bring a wheel loader to a construction site on a flat with end restraints."),
            profile("Industrial generator on riser flat", "Carry a boxed generator above the deck for forklift or crane extraction."),
            profile("Crawler dozer on depressed flat", "Keep the blade and tracks on a lowered center well during heavy equipment transfer."),
            profile("Transformer on reinforced flat", "Support a dense finned transformer on a stronger deck for a utility customer."),
        ],
        [
            profile("Low beam flat", "Set structural sections low to preserve vertical clearance to a fabricator."),
            profile("Bulkhead beam flat", "Restrain long steel sections inside tall end stops before departure from a mill."),
            profile("Raised-end beam flat", "Keep the lower beam faces supported for crane sling access at unloading."),
            profile("Depressed-deck beam flat", "Use the low deck center to bring a taller assembled beam through tight clearance."),
            profile("Extended structural-steel flat", "Carry long structural sections on a lengthened reinforced deck."),
        ],
        [
            profile("Low timber deck", "Carry small unprocessed logs from a loading point toward a mill."),
            profile("Bulkhead log flat", "Use tall ends to contain a longer log bundle between loading yard and mill."),
            profile("Raised-end logging flat", "Keep the ends of irregular logs supported on a stand-off deck."),
            profile("Low-center log flat", "Hold a tall stack lower on a route with limited overhead clearance."),
            profile("Reinforced heavy log flat", "Carry heavier timber on a longer deck for the larger mill intake."),
        ],
    ],
    // TTX's open, bi-level, tri-level, articulated, and tall-vehicle movements.
    [
        [
            profile("Compact open bi-level rack", "Carry a small return-season batch of finished vehicles on a heritage rack."),
            profile("Standard open bi-level rack", "Use two open decks to move finished vehicles on a low-volume distribution run."),
            profile("Extended open bi-level rack", "Add usable deck length for longer cars on an open heritage carrier."),
            profile("Reinforced open bi-level rack", "Brace both decks for a heavy mix of finished cars and utility vehicles."),
            profile("Tall-post open bi-level rack", "Use greater deck separation for larger vehicles in open-sided service."),
        ],
        [
            profile("Compact enclosed utility rack", "Protect a local shipment of pickups and sport utility vehicles on two levels."),
            profile("Standard vented SUV rack", "Shelter a two-deck SUV load and ventilate the enclosed body in transit."),
            profile("Long utility vehicle rack", "Use long enclosed sides for more trucks per departure from the assembly plant."),
            profile("Stiffened pickup rack", "Use reinforced side panels around the larger truck deck openings."),
            profile("High-roof enclosed pickup rack", "Leave more overhead room above tall utility vehicles and retain weather protection."),
        ],
        [
            profile("Short passenger-car tri-level", "Fit small cars on all three decks while serving a smaller unloading yard."),
            profile("Standard passenger-car tri-level", "Transfer factory passenger cars on three enclosed decks to a regional ramp."),
            profile("Extended enclosed tri-level", "Carry more compact cars per departure where a longer rack clears the route."),
            profile("Reinforced passenger-car tri-level", "Support all three car decks inside a stiffer enclosed rack frame."),
            profile("High-clearance passenger-car tri-level", "Give the top shelf added clearance for taller compact vehicles."),
        ],
        [
            profile("Short-joint articulated rack", "Move finished vehicles in an enclosed two-unit rack on tighter approaches."),
            profile("Standard articulated vehicle rack", "Keep an articulated pair together between the assembly plant and ramp."),
            profile("Extended articulated vehicle rack", "Carry a longer vehicle block using a joined bi-level rack."),
            profile("Reinforced articulated rack", "Use a strengthened joined frame for longer distribution runs."),
            profile("High-roof articulated rack", "Add upper clearance within a joined rack for a more varied vehicle load."),
        ],
        [
            profile("Short tall-vehicle carrier", "Move a small shipment of high-roof vehicles between a factory and unloading ramp."),
            profile("Standard high-roof vehicle carrier", "Give high vans or light trucks generous roof clearance behind enclosed panels."),
            profile("Extended tall-vehicle enclosure", "Carry longer tall vehicles that need additional deck length."),
            profile("Reinforced tall-vehicle rack", "Support heavy finished vehicles inside a more rigid roof and side frame."),
            profile("Maximum-clearance truck carrier", "Use the highest rail-cleared carrier for special tall vehicles."),
        ],
    ],
    // BNSF open hoppers and steel-service gondolas, including distinct exposed loading.
    [
        [
            profile("Compact two-bay coal hopper", "Deliver a small loose-coal shipment with two gravity discharge chutes."),
            profile("Standard three-bay coal hopper", "Feed a coal loader from three discharge hoppers along a unit train."),
            profile("Long four-bay coal hopper", "Load more lower-density coal into a lengthened four-bay open body."),
            profile("Heavy-ribbed coal hopper", "Support a tall coal load with strengthened side ribs and three outlets."),
            profile("Wide-lip high-capacity coal hopper", "Use the largest open coal bin shape with a reinforced top rim."),
        ],
        [
            profile("Compact ballast hopper", "Carry crushed stone in short batches to a small aggregate distributor."),
            profile("Standard three-bay stone hopper", "Discharge quarry stone at a construction yard from three gravity outlets."),
            profile("Extended aggregate hopper", "Deliver more crushed aggregate per trip to a long handling terminal."),
            profile("Heavy-ribbed stone hopper", "Strengthen the bin when handling repeated loads of quarry rock."),
            profile("Wide-lip aggregate hopper", "Use a broader bin and strong top lip for frequent loader fills at the pit."),
        ],
        [
            profile("Short mineral-ore hopper", "Keep dense ore on a short two-bay underframe for a mine spur."),
            profile("Standard ore hopper", "Deliver dense ore from the mine to a mill or ship loading pier."),
            profile("Long low-volume ore hopper", "Distribute ore along a longer underframe without a taller coal bin."),
            profile("Reinforced mineral hopper", "Resist frequent loading impact and dense ore residues with added ribs."),
            profile("Heavy-rim ore hopper", "Reinforce the bin edge against impact from ore-loading machinery."),
        ],
        [
            profile("Short scrap gondola", "Load two loose scrap heaps at a small recycler for crane unloading at the mill."),
            profile("Standard loose-scrap gondola", "Consolidate several scrap piles in an open car for remelting."),
            profile("Long scrap-service gondola", "Carry larger irregular scrap pieces that need extra open-floor length."),
            profile("Reinforced scrap gondola", "Use added ribs to withstand frequent loose steel loading and crane handling."),
            profile("High-side scrap gondola", "Keep tall recyclable steel fragments inside a reinforced top rim."),
        ],
        [
            profile("Short low-sided plate gondola", "Give a crane side access to a compact stack of mill plate."),
            profile("Standard low-sided plate gondola", "Ship layered plate bundles from a mill to a metal fabricator."),
            profile("Long steel-plate gondola", "Support long plates without crowding their ends against the gondola walls."),
            profile("Heavy-ribbed plate gondola", "Carry a denser plate stack inside reinforced side walls."),
            profile("Extended reinforced plate gondola", "Use the longest load bed here for large mill plate shipments."),
        ],
    ],
    // TTX centerbeam load support for lumber and other bundled building products.
    [
        [
            profile("Short empty centerbeam truss", "Return the short open truss to a lumber loading spur after its delivery."),
            profile("Standard empty centerbeam truss", "Reposition a general centerbeam for the next balanced bundle loading."),
            profile("Long lattice centerbeam return", "Return the extended cross-braced lumber frame without its previous load."),
            profile("Solid-spine centerbeam return", "Send a continuous center support back to a building-product terminal."),
            profile("Tall-bulkhead centerbeam return", "Return tall end guards and the open center brace to a mill yard."),
        ],
        [
            profile("Compact wrapped-lumber truss", "Carry two groups of weather-wrapped lumber packs to a small yard."),
            profile("Standard wrapped-lumber centerbeam", "Balance wrapped lumber bundles on both sides of the center truss."),
            profile("Long wrapped-lumber truss", "Give long stacks of wrapped lumber extra open-deck support."),
            profile("Solid-spine wrapped-lumber car", "Keep balanced wrapped bundles against a continuous central support."),
            profile("Tall-bulkhead wrapped-lumber car", "Use higher end support for tall wrapped lumber packages."),
        ],
        [
            profile("Short framing-stock centerbeam", "Serve a small dealer with bundles of dimensional framing studs."),
            profile("Standard framing-stock truss", "Brace regular-length dimensional studs against the center partition."),
            profile("Long framing-stock centerbeam", "Use an extended beam for multiple mill cuts of dimensional lumber."),
            profile("Solid-spine studs carrier", "Support numerous framing packs against a single continuous spine."),
            profile("Tall-load framing-stock centerbeam", "Restrain higher stacks of studs between tall bulkheads."),
        ],
        [
            profile("Compact wallboard centerbeam", "Carry a small balanced pair of wallboard stacks to a construction supplier."),
            profile("Standard wallboard truss", "Secure dimensional wallboard bundles along both sides of an open spine."),
            profile("Long wallboard centerbeam", "Give longer construction panels additional flat support along the frame."),
            profile("Solid-spine wallboard car", "Hold flat wallboard bundles against a continuous central support."),
            profile("Tall-bulkhead wallboard centerbeam", "Retain a taller stack of bundled wallboard inside taller end supports."),
        ],
        [
            profile("Compact packaged-tie carrier", "Take short batches of replacement ties to a track-maintenance yard."),
            profile("Standard packaged-tie centerbeam", "Restrain matching tie bundles on both sides of a lumber truss."),
            profile("Long high-count tie carrier", "Distribute more packages of railroad ties along an extended loading deck."),
            profile("Solid-spine tie carrier", "Retain rough rectangular tie bundles against a continuous spine."),
            profile("Tall-bulkhead tie carrier", "Use higher end walls to restrain tied timber packs bound for railroad maintenance."),
        ],
    ],
    // Union Pacific open, covered, single-trough and transverse steel-coil classes.
    [
        [
            profile("Short open longitudinal coil cradle", "Carry a pair of small exposed hot-rolled coils along the central load trough."),
            profile("Stiff-sill open coil cradle", "Support two exposed mill coils behind a reinforced side sill."),
            profile("Long open coil well", "Give three exposed hot-rolled coils space along a longer unshielded cradle."),
            profile("High-sill hot-coil car", "Carry three open hot-rolled coils on the higher center deck."),
            profile("Heavy open coil carrier", "Take dense weather-tolerant coils in a heavy-duty unshielded frame."),
        ],
        [
            profile("Compact single covered trough", "Shield one small weather-sensitive steel coil from rain and residue."),
            profile("Reinforced single-hood coil car", "Carry a single protected coil in a sturdier cradle between mill and processor."),
            profile("Extended single-hood coil car", "Use a longer sealed cover for a large width of finish-sensitive coil."),
            profile("High-sill single covered trough", "Use a raised side support under one protected shipment of rolled steel."),
            profile("Heavy single-hood coil car", "Protect a dense individual coil on a heavy deck for a fabricator."),
        ],
        [
            profile("Compact two-cover coil car", "Separate two smaller weather-sensitive coils with individually removable hoods."),
            profile("Reinforced paired-coil car", "Support two protected dense coils in separate positions on a stiffened sill."),
            profile("Long paired-coil carrier", "Give a pair of longer covered coils enough trough room for crane handling."),
            profile("Raised-sill twin hood car", "Separate two protected coils above a higher deck structure."),
            profile("Heavy paired-coil carrier", "Use a stronger floor for a matched pair of dense, individually covered coils."),
        ],
        [
            profile("Compact triple-hood coil car", "Move three smaller separately covered steel orders out of a coil depot."),
            profile("Reinforced triple-cradle car", "Stiffen the car under three sheltered coil cradles."),
            profile("Extended triple-cover coil car", "Spread three enclosed coils along a longer deck for separate unloading."),
            profile("High-sill triple-cover coil car", "Carry three sheltered coils over a raised deck and side sill."),
            profile("Heavy segmented coil carrier", "Keep three dense mid-size coils in distinct weather-protected saddles."),
        ],
        [
            profile("Short transverse coil car", "Turn a heavy exposed coil across the deck and brace it in a short cradle."),
            profile("Reinforced transverse coil car", "Carry a transverse coil with a stronger side sill supporting the cradle."),
            profile("Extended transverse coil carrier", "Give one heavier crosswise roll additional deck room during mill transfer."),
            profile("High-sill transverse cradle", "Use a lifted crosswise saddle under a heavy coil with ample ground clearance."),
            profile("Heavy transverse coil cradle", "Support the largest exposed crosswise roll in a deep reinforced saddle."),
        ],
    ],
    // Greenbrier mechanical refrigerated profiles and BNSF perishables practice.
    [
        [
            profile("Compact end-unit produce reefer", "Cool a local run from a small produce packer to a distribution market."),
            profile("Standard end-unit produce reefer", "Keep fresh produce within its cold chain between packing and wholesale."),
            profile("Wide-door produce reefer", "Use the larger opening to reduce loading delays for delicate produce pallets."),
            profile("Low-mount produce reefer", "Set machinery low on the frame while retaining produce loading clearance."),
            profile("High-cube produce reefer", "Leave refrigeration airflow above a tall light produce order."),
        ],
        [
            profile("Compact freezer reefer", "Maintain a small load of frozen goods from plant to warehouse."),
            profile("Standard end-unit freezer reefer", "Hold a stable frozen load during regular service to a distribution hub."),
            profile("Wide-door freezer reefer", "Load large batches of frozen products through a wide sealed opening."),
            profile("Low-mount freezer reefer", "Keep the refrigeration machine low for a large uninterrupted end profile."),
            profile("High-cube freezer reefer", "Use the taller insulated body for high-volume frozen warehouse freight."),
        ],
        [
            profile("Compact chilled-meat reefer", "Keep a short shipment of chilled meat at its required temperature."),
            profile("Standard chilled-meat end-unit car", "Move packaged chilled meat between processor and cold store."),
            profile("Wide-door chilled-meat reefer", "Clear width for packaged meat pallets while maintaining a plug-door seal."),
            profile("Low-mount chilled-meat reefer", "Use a low-mounted refrigeration module for a compact chilled service."),
            profile("High-cube chilled-meat reefer", "Carry bulk chilled orders in a larger insulated hold with overhead air space."),
        ],
        [
            profile("Compact dairy reefer", "Pick up a small dairy lot for a short regional cold-chain run."),
            profile("Standard dairy end-unit car", "Keep dairy refrigerated between packing and a distribution facility."),
            profile("Wide-door dairy reefer", "Use an extended refrigerated body for pallets loaded across a larger opening."),
            profile("Low-mount dairy reefer", "Allow a low machinery installation while retaining the tall side loading door."),
            profile("High-cube dairy reefer", "Hold tall packaged dairy shipments in a higher-volume insulated body."),
        ],
        [
            profile("Compact temperature-controlled beverage car", "Protect a smaller shipment of sensitive drinks from outdoor temperature swings."),
            profile("Standard beverage end-unit reefer", "Keep specialty beverages stable during a cold transfer between warehouses."),
            profile("Wide-door cold beverage car", "Load large specialty drink pallets through the wide insulated entry."),
            profile("Low-mount beverage reefer", "Retain a clear end profile around a lower-mounted cooling module."),
            profile("High-cube beverage reefer", "Accommodate tall insulated beverage pallets for a distribution hub."),
        ],
    ],
];
