"""Extract Scrivener project content to structured markdown folders."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from doc_tools.docx_reader import read_docx_text
from doc_tools.markdown_writer import slugify, write_markdown
from doc_tools.scrivener.binder import BinderItem, ScrivenerProject, parse_binder_from_scriv


# Map binder item types to output folder names
FOLDER_MAP = {
    "DraftFolder": "manuscript",
    "ResearchFolder": "research",
    "TrashFolder": None,  # Skip trash
}

# Special folder titles to output folder names
TITLE_MAP = {
    "characters": "characters",
    "places": "places",
    "locations": "places",
    "notes": "notes",
    "front matter": "front-matter",
    "template sheets": None,  # Skip templates
}


def extract_scrivener_project(
    scriv_path: Path,
    output_dir: Path,
    *,
    include_notes: bool = True,
    include_synopsis: bool = True,
) -> dict[str, int]:
    """Extract a Scrivener project to structured markdown.

    Args:
        scriv_path: Path to the .scriv directory.
        output_dir: Directory to write extracted content.
        include_notes: Whether to include document notes.
        include_synopsis: Whether to include document synopses.

    Returns:
        Dict of category -> count of extracted documents.
    """
    project = parse_binder_from_scriv(scriv_path)
    data_dir = scriv_path / "Files" / "Data"

    stats: dict[str, int] = {}

    for item in project.root_items:
        # Determine output folder
        folder_name = _get_folder_name(item)
        if folder_name is None:
            continue

        folder_path = output_dir / folder_name
        count = _extract_item_tree(
            item,
            data_dir,
            folder_path,
            include_notes=include_notes,
            include_synopsis=include_synopsis,
        )
        stats[folder_name] = stats.get(folder_name, 0) + count

    return stats


def _get_folder_name(item: BinderItem) -> str | None:
    """Determine output folder name for a binder item."""
    # Check type-based mapping first
    if item.item_type in FOLDER_MAP:
        return FOLDER_MAP[item.item_type]

    # Check title-based mapping
    title_lower = item.title.lower()
    if title_lower in TITLE_MAP:
        return TITLE_MAP[title_lower]

    # Default: use slugified title for folders
    if item.item_type == "Folder":
        return slugify(item.title)

    return None


def _extract_item_tree(
    item: BinderItem,
    data_dir: Path,
    output_dir: Path,
    *,
    include_notes: bool = True,
    include_synopsis: bool = True,
    depth: int = 0,
) -> int:
    """Recursively extract an item and its children.

    Returns count of extracted documents.
    """
    count = 0

    # Extract this item's content if it has any
    if item.item_type in ("Text", "Folder"):
        extracted = _extract_item_content(
            item,
            data_dir,
            output_dir,
            include_notes=include_notes,
            include_synopsis=include_synopsis,
        )
        if extracted:
            count += 1

    # Handle children
    if item.children:
        # If this item has children, create a subfolder for them
        if item.item_type == "Folder" and item.title:
            child_output = output_dir / slugify(item.title)
        else:
            child_output = output_dir

        for i, child in enumerate(item.children, 1):
            child_count = _extract_item_tree(
                child,
                data_dir,
                child_output,
                include_notes=include_notes,
                include_synopsis=include_synopsis,
                depth=depth + 1,
            )
            count += child_count

    return count


def _extract_item_content(
    item: BinderItem,
    data_dir: Path,
    output_dir: Path,
    *,
    include_notes: bool = True,
    include_synopsis: bool = True,
) -> bool:
    """Extract content for a single item.

    Returns True if content was extracted.
    """
    item_data_dir = data_dir / item.uuid
    if not item_data_dir.exists():
        return False

    content_file = item_data_dir / "content.docx"
    notes_file = item_data_dir / "notes.docx"
    synopsis_file = item_data_dir / "synopsis.docx"

    # Build markdown content
    parts: list[str] = []

    # Add title as heading
    if item.title:
        parts.append(f"# {item.title}")

    # Add synopsis if present
    if include_synopsis and synopsis_file.exists():
        synopsis = read_docx_text(synopsis_file).strip()
        if synopsis:
            parts.append(f"*{synopsis}*")

    # Add main content
    if content_file.exists():
        content = read_docx_text(content_file).strip()
        if content:
            parts.append(content)

    # Add notes if present
    if include_notes and notes_file.exists():
        notes = read_docx_text(notes_file).strip()
        if notes:
            parts.append("---")
            parts.append("## Notes")
            parts.append(notes)

    if not parts or (len(parts) == 1 and parts[0].startswith("# ")):
        # Only title, no real content
        return False

    # Write markdown file
    filename = slugify(item.title) if item.title else item.uuid
    output_path = output_dir / f"{filename}.md"
    write_markdown("\n\n".join(parts), output_path)

    return True


def main() -> int:
    """CLI entry point for Scrivener extraction."""
    parser = argparse.ArgumentParser(
        description="Extract Scrivener project to structured markdown"
    )
    parser.add_argument(
        "scriv_path",
        type=Path,
        help="Path to .scriv directory",
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=None,
        help="Output directory (default: same location as .scriv with -extracted suffix)",
    )
    parser.add_argument(
        "--no-notes",
        action="store_true",
        help="Don't include document notes",
    )
    parser.add_argument(
        "--no-synopsis",
        action="store_true",
        help="Don't include document synopses",
    )

    args = parser.parse_args()

    scriv_path = args.scriv_path.resolve()
    if not scriv_path.exists():
        print(f"Error: {scriv_path} does not exist", file=sys.stderr)
        return 1

    if args.output:
        output_dir = args.output.resolve()
    else:
        # Default: create directory next to .scriv
        name = scriv_path.stem.replace(".scriv", "")
        output_dir = scriv_path.parent / f"{slugify(name)}-extracted"

    print(f"Extracting {scriv_path.name} to {output_dir}")

    stats = extract_scrivener_project(
        scriv_path,
        output_dir,
        include_notes=not args.no_notes,
        include_synopsis=not args.no_synopsis,
    )

    print("\nExtracted:")
    for folder, count in sorted(stats.items()):
        print(f"  {folder}: {count} documents")

    return 0


if __name__ == "__main__":
    sys.exit(main())
