"""Tests for the name parser — uses saved HTML fixtures, no network calls."""

from name_harvester.harvester import parse_names, GENRE_MAPPINGS


SAMPLE_HTML = """
<html>
<body>
<div id="result">
  Vasil<br>
  Ilyana<br>
  Pyotir<br>
  Maren<br>
</div>
</body>
</html>
"""


def test_parse_names_extracts_from_result_div():
    names = parse_names(SAMPLE_HTML)
    assert len(names) == 4
    assert "Vasil" in names
    assert "Pyotir" in names


def test_parse_names_returns_empty_for_missing_div():
    names = parse_names("<html><body><p>No names here</p></body></html>")
    assert names == []


def test_parse_names_filters_short_strings():
    html = '<html><body><div id="result">A<br>Bo<br>Cat</div></body></html>'
    names = parse_names(html)
    assert "A" not in names
    assert "Bo" in names
    assert "Cat" in names


def test_genre_mappings_have_required_fields():
    assert len(GENRE_MAPPINGS) >= 1
    for m in GENRE_MAPPINGS:
        assert m.genre_id
        assert m.url.startswith("https://")
        assert m.source_description
