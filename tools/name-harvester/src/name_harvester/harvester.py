"""Polite name harvesting from fantasynamegenerators.com."""

import time
from dataclasses import dataclass

import httpx
from bs4 import BeautifulSoup


@dataclass
class GenreNameMapping:
    """Maps a storyteller genre to a fantasynamegenerators.com URL."""
    genre_id: str
    url: str
    source_description: str


GENRE_MAPPINGS: list[GenreNameMapping] = [
    GenreNameMapping(
        genre_id="low_fantasy_folklore",
        url="https://www.fantasynamegenerators.com/slavic-names.php",
        source_description="fantasynamegenerators.com/slavic-names",
    ),
    GenreNameMapping(
        genre_id="sci_fi_noir",
        url="https://www.fantasynamegenerators.com/cyberpunk-names.php",
        source_description="fantasynamegenerators.com/cyberpunk-names",
    ),
    GenreNameMapping(
        genre_id="cozy_ghost_story",
        url="https://www.fantasynamegenerators.com/english-names.php",
        source_description="fantasynamegenerators.com/english-names",
    ),
]


def harvest_names(url: str, delay: float = 2.5) -> list[str]:
    """Fetch names from a fantasynamegenerators.com page. Rate-limited."""
    time.sleep(delay)
    client = httpx.Client(
        timeout=15.0,
        headers={"User-Agent": "storyteller-name-harvester/0.1 (research tool)"},
    )
    response = client.post(url, data={})
    response.raise_for_status()
    return parse_names(response.text)


def parse_names(html: str) -> list[str]:
    """Extract names from the generator result HTML."""
    soup = BeautifulSoup(html, "html.parser")
    result_div = soup.find("div", id="result") or soup.find("div", class_="nameList")
    if not result_div:
        return []
    names = []
    for item in result_div.stripped_strings:
        name = item.strip()
        if name and len(name) > 1:
            names.append(name)
    return names
