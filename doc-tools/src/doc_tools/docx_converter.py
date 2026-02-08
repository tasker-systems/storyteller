"""Convert DOCX files to structured markdown."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from doc_tools.docx_reader import (
    BreakType,
    DocumentBreak,
    DocumentContent,
    Paragraph,
    ParagraphType,
    read_docx,
)
from doc_tools.markdown_writer import (
    content_to_markdown,
    slugify,
    write_markdown,
    write_sections_as_files,
)


def convert_docx_to_markdown(
    docx_path: Path,
    output_dir: Path,
    *,
    split_chapters: bool = True,
    min_sections_for_split: int = 3,
) -> list[Path]:
    """Convert a DOCX file to markdown.

    Args:
        docx_path: Path to the DOCX file.
        output_dir: Directory to write output files.
        split_chapters: Whether to attempt splitting into chapter files.
        min_sections_for_split: Minimum sections needed to split (otherwise single file).

    Returns:
        List of paths to created files.
    """
    content = read_docx(docx_path)

    # Try to split by page breaks first
    sections = content.sections()

    if split_chapters and len(sections) >= min_sections_for_split:
        # We have enough sections to split
        return write_sections_as_files(
            sections,
            output_dir,
            default_prefix="chapter",
        )

    # Try splitting by headings instead
    heading_sections = _split_by_headings(content)
    if split_chapters and len(heading_sections) >= min_sections_for_split:
        return write_sections_as_files(
            heading_sections,
            output_dir,
            default_prefix="chapter",
        )

    # Fall back to single file
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / f"{slugify(docx_path.stem)}.md"
    md = content_to_markdown(content)
    write_markdown(md, output_path)
    return [output_path]


def _split_by_headings(content: DocumentContent) -> list[list[Paragraph]]:
    """Split content by heading paragraphs.

    Groups paragraphs into sections starting at each Heading1 or Title.
    """
    sections: list[list[Paragraph]] = []
    current_section: list[Paragraph] = []

    for elem in content.elements:
        if isinstance(elem, DocumentBreak):
            continue

        if isinstance(elem, Paragraph):
            # Start new section on major headings
            if elem.paragraph_type in (ParagraphType.HEADING1, ParagraphType.TITLE):
                if current_section:
                    sections.append(current_section)
                current_section = [elem]
            else:
                current_section.append(elem)

    if current_section:
        sections.append(current_section)

    return sections


def analyze_document(docx_path: Path) -> dict:
    """Analyze a DOCX document structure.

    Returns info about paragraphs, styles, and breaks.
    """
    content = read_docx(docx_path)

    # Count elements by type
    para_count = 0
    page_breaks = 0
    section_breaks = 0
    styles: dict[str, int] = {}
    para_types: dict[str, int] = {}

    for elem in content.elements:
        if isinstance(elem, DocumentBreak):
            if elem.break_type == BreakType.PAGE:
                page_breaks += 1
            elif elem.break_type == BreakType.SECTION:
                section_breaks += 1
        elif isinstance(elem, Paragraph):
            para_count += 1
            type_name = elem.paragraph_type.name
            para_types[type_name] = para_types.get(type_name, 0) + 1
            if elem.style_name:
                styles[elem.style_name] = styles.get(elem.style_name, 0) + 1

    sections = content.sections()
    heading_sections = _split_by_headings(content)

    return {
        "paragraphs": para_count,
        "page_breaks": page_breaks,
        "section_breaks": section_breaks,
        "sections_by_page_break": len(sections),
        "sections_by_heading": len(heading_sections),
        "paragraph_types": para_types,
        "styles": styles,
    }


def main() -> int:
    """CLI entry point for DOCX conversion."""
    parser = argparse.ArgumentParser(description="Convert DOCX file to structured markdown")
    parser.add_argument(
        "docx_path",
        type=Path,
        help="Path to DOCX file",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="Output directory (default: same location as DOCX)",
    )
    parser.add_argument(
        "--single-file",
        action="store_true",
        help="Output as single markdown file instead of splitting",
    )
    parser.add_argument(
        "--analyze",
        action="store_true",
        help="Analyze document structure instead of converting",
    )

    args = parser.parse_args()

    docx_path = args.docx_path.resolve()
    if not docx_path.exists():
        print(f"Error: {docx_path} does not exist", file=sys.stderr)
        return 1

    if args.analyze:
        info = analyze_document(docx_path)
        print(f"Document: {docx_path.name}")
        print(f"  Paragraphs: {info['paragraphs']}")
        print(f"  Page breaks: {info['page_breaks']}")
        print(f"  Section breaks: {info['section_breaks']}")
        print(f"  Sections (by page break): {info['sections_by_page_break']}")
        print(f"  Sections (by heading): {info['sections_by_heading']}")
        print("\n  Paragraph types:")
        for ptype, count in sorted(info["paragraph_types"].items()):
            print(f"    {ptype}: {count}")
        if info["styles"]:
            print("\n  Styles used:")
            for style, count in sorted(info["styles"].items()):
                print(f"    {style}: {count}")
        return 0

    if args.output:
        output_dir = args.output.resolve()
    else:
        output_dir = docx_path.parent / slugify(docx_path.stem)

    print(f"Converting {docx_path.name}")

    paths = convert_docx_to_markdown(
        docx_path,
        output_dir,
        split_chapters=not args.single_file,
    )

    print(f"\nCreated {len(paths)} file(s) in {output_dir}")
    for path in paths:
        print(f"  {path.name}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
