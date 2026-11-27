"""Reviewed Strip components tied to their OpenStreetMap footprints."""

from dataclasses import dataclass
from enum import IntEnum


class Material(IntEnum):
    LIMESTONE = 21
    IVORY = 22
    ROSE = 23
    SLATE_GLASS = 24
    BRONZE_GLASS = 25
    TERRACOTTA = 26
    CREAM_BLUE_ROOF = 27
    GOLD_GLASS = 28
    DESERT_STUCCO = 29
    PALE_CONCRETE = 30


@dataclass(frozen=True)
class Component:
    osm_id: int
    property_name: str
    name: str
    material: Material
    osm_type: str = "way"
    height_m: int | None = None
    height_note: str | None = None
    roof_profile: str | None = None


SOURCE_REFERENCES = {
    "The Venetian": (
        "https://www.watg.com/project/venetian-las-vegas-usa/",
        "https://www.venetianlasvegas.com/media/press-kit/photos.html",
    ),
    "The Palazzo": (
        "https://www.venetianlasvegas.com/media/press-kit/photos.html",
        "https://www.vanderweil.com/project/venetian-resort-hotel-casino-the-palazzo/",
    ),
    "Flamingo": (
        "https://newsroom.caesars.com/Property-Fact-Sheets/Flamingo-Las-Vegas/default.aspx",
        "https://special.library.unlv.edu/ark%3A/62930/d1wh2dn67",
    ),
    "The LINQ": (
        "https://newsroom.caesars.com/Property-Fact-Sheets/The-LINQ-Hotel--Experience/default.aspx",
    ),
    "Horseshoe": (
        "https://newsroom.caesars.com/Property-Fact-Sheets/Horseshoe-Las-Vegas/default.aspx",
        "https://www.businesswire.com/news/home/20230327005624/en/Horseshoe-Las-Vegas-Celebrates-its-Grand-Opening-on-the-Las-Vegas-Strip",
    ),
    "Paris": (
        "https://newsroom.caesars.com/Property-Fact-Sheets/Paris-Las-Vegas/",
        "https://www.bwaltd.com/project-detail/versailles-tower-remodel",
    ),
    "Bellagio": (
        "https://www.jerde.com/projects/7501/bellagio",
        "https://bellagio.mgmresorts.com/en/contact-us.html",
    ),
    "The Cosmopolitan": (
        "https://arquitectonica.com/architecture/project/cosmopolitan/",
        "https://www.cdc-usa.com/projects/cosmopolitan",
    ),
    "Caesars Palace": (
        "https://newsroom.caesars.com/media-resources/property-fact-sheets/default.aspx",
    ),
    "Planet Hollywood": (
        "https://newsroom.caesars.com/Property-Fact-Sheets/Planet-Hollywood-Resort--Casino/default.aspx",
    ),
}


REVIEWED_BUILDINGS = {
    (building.osm_type, building.osm_id): building
    for building in (
        Component(7770314, "The Venetian", "Venetian podium", Material.LIMESTONE, osm_type="relation"),
        Component(30618090, "The Venetian", "Venetian Tower", Material.IVORY),
        Component(111322067, "The Venetian", "Venezia Tower", Material.IVORY),
        Component(43875964, "The Venetian", "Campanile", Material.TERRACOTTA, roof_profile="pyramidal"),
        Component(111367819, "The Venetian", "Venetian Expo", Material.PALE_CONCRETE),
        Component(180584269, "The Palazzo", "Palazzo podium", Material.LIMESTONE),
        Component(30424125, "The Palazzo", "Palazzo Tower", Material.GOLD_GLASS),
        Component(116761428, "Flamingo", "Flamingo casino podium", Material.ROSE),
        Component(115766957, "Flamingo", "Flamingo north tower", Material.ROSE),
        Component(115766962, "Flamingo", "Flamingo south tower", Material.ROSE),
        Component(116761035, "Flamingo", "Flamingo west wing", Material.ROSE),
        Component(116761280, "Flamingo", "Flamingo west roof section", Material.ROSE),
        Component(310585854, "Flamingo", "Flamingo low-rise wing", Material.PALE_CONCRETE),
        Component(115771295, "The LINQ", "LINQ casino podium", Material.PALE_CONCRETE),
        Component(115773426, "The LINQ", "LINQ hotel tower", Material.SLATE_GLASS),
        Component(
            116867081, "Horseshoe", "Horseshoe podium", Material.PALE_CONCRETE,
            height_m=16, height_note="Estimated from adjacent five-storey OSM parts. The enclosing way carries the resort tower floor count.",
        ),
        Component(27897922, "Horseshoe", "Horseshoe Resort Tower", Material.PALE_CONCRETE),
        Component(1582510990, "Horseshoe", "Horseshoe east podium", Material.PALE_CONCRETE),
        Component(1582510992, "Horseshoe", "Horseshoe west podium", Material.IVORY),
        Component(1582510991, "Horseshoe", "Horseshoe east pavilion", Material.PALE_CONCRETE),
        Component(33974140, "Paris", "Paris casino podium", Material.LIMESTONE),
        Component(27831697, "Paris", "Paris main tower", Material.CREAM_BLUE_ROOF),
        Component(27897921, "Paris", "Versailles Tower", Material.CREAM_BLUE_ROOF),
        Component(1599958, "Bellagio", "Bellagio podium", Material.LIMESTONE, osm_type="relation"),
        Component(25723909, "Bellagio", "Bellagio main tower", Material.CREAM_BLUE_ROOF),
        Component(114703388, "Bellagio", "Bellagio domed crown", Material.CREAM_BLUE_ROOF, roof_profile="dome"),
        Component(25723910, "Bellagio", "Bellagio Spa Tower base", Material.LIMESTONE),
        Component(114716301, "Bellagio", "Bellagio Spa Tower", Material.CREAM_BLUE_ROOF),
        Component(114714407, "Bellagio", "Bellagio north entrance roof", Material.LIMESTONE),
        Component(114718351, "Bellagio", "Bellagio Conservatory roof", Material.PALE_CONCRETE),
        Component(27897905, "The Cosmopolitan", "Cosmopolitan Chelsea Tower", Material.SLATE_GLASS),
        Component(134795273, "The Cosmopolitan", "Cosmopolitan Boulevard Tower", Material.SLATE_GLASS),
        Component(472669744, "The Cosmopolitan", "Cosmopolitan Boulevard low-rise", Material.SLATE_GLASS),
        Component(1176928048, "Caesars Palace", "Augustus Tower", Material.IVORY),
        Component(126267789, "Caesars Palace", "Octavius Tower", Material.IVORY),
        Component(134921373, "Planet Hollywood", "Planet Hollywood main tower", Material.PALE_CONCRETE),
    )
}


# Open structures have their own renderer over the retained podium or ground.
OPEN_STRUCTURE_WAYS = {
    114697072, 114697073, 114697074,  # Arc de Triomphe piers and span
    364846544,  # Paris balloon sign
}
