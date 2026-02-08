"""Parse Scrivener binder.backup XML to extract project structure."""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path
from zipfile import ZipFile, is_zipfile

from lxml import etree


@dataclass
class BinderItem:
    """A single item in the Scrivener binder hierarchy."""

    uuid: str
    title: str
    item_type: str
    children: list[BinderItem] = field(default_factory=list)
    include_in_compile: bool = True

    def walk(self) -> Iterator[BinderItem]:
        """Depth-first traversal of this item and all descendants."""
        yield self
        for child in self.children:
            yield from child.walk()

    def find_by_type(self, item_type: str) -> Iterator[BinderItem]:
        """Find all items of a specific type."""
        for item in self.walk():
            if item.item_type == item_type:
                yield item

    def find_by_uuid(self, uuid: str) -> BinderItem | None:
        """Find an item by UUID."""
        for item in self.walk():
            if item.uuid == uuid:
                return item
        return None


@dataclass
class ScrivenerProject:
    """Parsed Scrivener project structure."""

    identifier: str
    version: str
    creator: str
    root_items: list[BinderItem] = field(default_factory=list)

    @property
    def manuscript(self) -> BinderItem | None:
        """Get the manuscript/draft folder."""
        for item in self.root_items:
            if item.item_type == "DraftFolder":
                return item
        return None

    @property
    def research(self) -> BinderItem | None:
        """Get the research folder."""
        for item in self.root_items:
            if item.item_type == "ResearchFolder":
                return item
        return None

    @property
    def trash(self) -> BinderItem | None:
        """Get the trash folder."""
        for item in self.root_items:
            if item.item_type == "TrashFolder":
                return item
        return None

    def find_folder_by_title(self, title: str) -> BinderItem | None:
        """Find a top-level folder by title."""
        for item in self.root_items:
            if item.title.lower() == title.lower():
                return item
        return None

    def all_items(self) -> Iterator[BinderItem]:
        """Iterate over all items in the project."""
        for item in self.root_items:
            yield from item.walk()


def parse_binder(path: Path) -> ScrivenerProject:
    """Parse a Scrivener binder.backup XML file.

    Args:
        path: Path to binder.backup file (may be XML or zip-compressed XML).

    Returns:
        Parsed ScrivenerProject structure.
    """
    # Scrivener 3 uses zip-compressed binder files
    if is_zipfile(path):
        with ZipFile(path) as zf:
            # The zip contains a single XML file, usually named 'binder.scrivproj' or similar
            names = zf.namelist()
            if not names:
                raise ValueError(f"Empty zip file: {path}")
            # Read the first file in the archive
            with zf.open(names[0]) as f:
                tree = etree.parse(f)
    else:
        tree = etree.parse(path)

    root = tree.getroot()

    project = ScrivenerProject(
        identifier=root.get("Identifier", ""),
        version=root.get("Version", ""),
        creator=root.get("Creator", ""),
    )

    binder = root.find("Binder")
    if binder is not None:
        for item_elem in binder.findall("BinderItem"):
            item = _parse_binder_item(item_elem)
            if item:
                project.root_items.append(item)

    return project


def _parse_binder_item(elem: etree._Element) -> BinderItem | None:
    """Parse a BinderItem element recursively."""
    uuid = elem.get("UUID")
    item_type = elem.get("Type")

    if not uuid or not item_type:
        return None

    title_elem = elem.find("Title")
    title = title_elem.text if title_elem is not None and title_elem.text else ""

    # Check include in compile
    include_in_compile = True
    metadata = elem.find("MetaData")
    if metadata is not None:
        include_elem = metadata.find("IncludeInCompile")
        if include_elem is not None and include_elem.text:
            include_in_compile = include_elem.text.lower() == "yes"

    item = BinderItem(
        uuid=uuid,
        title=title,
        item_type=item_type,
        include_in_compile=include_in_compile,
    )

    # Parse children
    children_elem = elem.find("Children")
    if children_elem is not None:
        for child_elem in children_elem.findall("BinderItem"):
            child = _parse_binder_item(child_elem)
            if child:
                item.children.append(child)

    return item


def parse_binder_from_scriv(scriv_path: Path) -> ScrivenerProject:
    """Parse a Scrivener project from its .scriv directory.

    Args:
        scriv_path: Path to the .scriv directory.

    Returns:
        Parsed ScrivenerProject structure.
    """
    binder_path = scriv_path / "Files" / "binder.backup"
    if not binder_path.exists():
        # Try autosave version
        binder_path = scriv_path / "Files" / "binder.autosave"

    if not binder_path.exists():
        raise FileNotFoundError(f"No binder file found in {scriv_path}")

    return parse_binder(binder_path)
