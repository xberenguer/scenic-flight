#!/usr/bin/env python3
"""Reconcile Nevada location records with the Strip and Downtown map labels."""

import argparse
from collections import Counter
import csv
import json
from pathlib import Path
import re
from xml.etree import ElementTree as XML
from zipfile import ZipFile

AREAS = {"Las Vegas Strip", "Downtown Las Vegas"}
GENERIC = {"and", "the", "hotel", "casino", "resort", "suites", "las", "vegas", "nv", "spa"}
SPORTS_BOOK = re.compile(r"(?:\s+\-\s+Sports Pool|\s*(?:\s-\s)?race(?: book)?\s+(?:and|&)\s+sports(?: pool)?)$", re.I)
ALIASES = {
    "Aztec Gold Inn & Casino": "Aztec Inn",
    "Main Street Station Hotel, Casino and Brewery": "Main Street Station",
    "Strat Hotel, Casino & Tower, The": "The Strat",
    "Strat, Hotel, Casino & Skypod, The": "The Strat",
    "Casino Royale": "Best Western Plus Casino Royale",
    "Horseshoe Las Vegas": "Horseshoe Las Vegas Hotel & Casino - A Caesars Rewards Destination",
    "Linq Hotel and Casino, The": "The LINQ Hotel + Experience",
    "Peppermill Coffee Shop and Lounge": "Peppermill",
    "Rio Hotel & Casino": "Rio All Suite Hotel and Casino",
    "The Casino At Virgin Hotels Las Vegas": "Virgin Hotels Las Vegas, Curio Collection by Hilton",
}
AIRPORT_CONCESSION = "Michael J. Gaughan Airport Slot Concession"
SHARED_BOOK = "Venetian Resort Hotel Casino/Palazzo Resort Hotel Casino"


def normalized(name):
    return " ".join(word for word in re.findall(r"[a-z0-9]+", name.casefold()) if word not in GENERIC)


def location_rows(report):
    with ZipFile(report) as workbook:
        root = XML.fromstring(workbook.read("xl/sharedStrings.xml"))
        words = ["".join(text.text or "" for text in row.findall(".//{*}t")) for row in root]
        sheet = XML.fromstring(workbook.read("xl/worksheets/sheet1.xml"))
    columns = None
    records = {}
    for row in sheet.findall(".//{*}row"):
        values = {}
        for cell in row:
            value = cell.findtext("{*}v")
            if value is not None:
                col = re.sub(r"\d+", "", cell.get("r", ""))
                values[col] = words[int(value)] if cell.get("t") == "s" else value
        if columns is None:
            fields = {value: key for key, value in values.items()}
            if {"Location Id", "Location Name", "County", "Area"} <= fields.keys():
                columns = [fields[key] for key in ("Location Id", "Location Name", "County", "Area")]
            continue
        location, name, county, area = (values.get(key, "") for key in columns)
        if county == "Clark" and area in AREAS and re.fullmatch(r"\d+-\d+", location):
            records[location] = (area, name)
    if not records:
        raise ValueError("no Strip or Downtown rows in the state report")
    return records


def reconcile(records, place_names):
    normal_names = {}
    for name in sorted(place_names, key=lambda value: (len(value), value)):
        normal_names.setdefault(normalized(name), name)
    alias_names = {normalized(name): mapped for name, mapped in ALIASES.items()}
    for key, (area, name) in sorted(records.items(), key=lambda item: item[1]):
        venue = SPORTS_BOOK.sub("", name).rstrip(" -")
        same_site = venue != name or name == AIRPORT_CONCESSION
        if name == AIRPORT_CONCESSION:
            labels = ["Harry Reid International Airport"]
        elif venue == SHARED_BOOK and same_site:
            labels = ["The Venetian Las Vegas", "The Palazzo"]
        else:
            match = alias_names.get(normalized(venue), normal_names.get(normalized(venue)))
            labels = [match] if match else []
        if name == "Tropicana Las Vegas":
            status = "Demolished site"
            labels = []
        elif name.startswith("Hard Rock Las Vegas"):
            status = "Tower under construction"
        elif not labels or any(label not in place_names for label in labels):
            status = "Needs location review"
            labels = []
        elif same_site:
            status = "Service at mapped venue"
        else:
            status = "Venue label matched"
        yield area, key, name, " | ".join(labels), status


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path, help="NGCB nonrestricted count report in XLSX format")
    parser.add_argument("catalog", type=Path, help="generated las-vegas.json")
    parser.add_argument("output", type=Path, help="audit TSV")
    args = parser.parse_args()
    places = {row["name"] for row in json.loads(args.catalog.read_text())["places"]}
    rows = list(reconcile(location_rows(args.report), places))
    with args.output.open("w", newline="") as output:
        writer = csv.writer(output, delimiter="\t", lineterminator="\n")
        writer.writerow(("NGCB area", "Location ID", "Official location name", "Mapped label", "Check"))
        writer.writerows(rows)
    for kind, count in sorted(Counter(row[-1] for row in rows).items()):
        print(f"{kind}\t{count}")


if __name__ == "__main__":
    main()
