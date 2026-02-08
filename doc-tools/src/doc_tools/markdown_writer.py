"""Markdown output generation from structured content."""

from __future__ import annotations

import re
from pathlib import Path

from doc_tools.docx_reader import DocumentContent, Paragraph, ParagraphType


def slugify(text: str) -> str:
    """Convert text to a filesystem-safe slug.

    Args:
        text: Input text to slugify.

    Returns:
        Lowercase slug with hyphens.
    """
    # Remove non-alphanumeric characters (except spaces and hyphens)
    text = re.sub(r"[^\w\s-]", "", text.lower())
    # Replace spaces and multiple hyphens with single hyphen
    text = re.sub(r"[-\s]+", "-", text)
    return text.strip("-")


def paragraph_to_markdown(para: Paragraph) -> str:
    """Convert a paragraph to markdown.

    Args:
        para: Paragraph to convert.

    Returns:
        Markdown string.
    """
    match para.paragraph_type:
        case ParagraphType.TITLE:
            return f"# {para.text}"
        case ParagraphType.HEADING1:
            return f"# {para.text}"
        case ParagraphType.HEADING2:
            return f"## {para.text}"
        case ParagraphType.HEADING3:
            return f"### {para.text}"
        case ParagraphType.HEADING4:
            return f"#### {para.text}"
        case ParagraphType.BODY:
            return para.text


def content_to_markdown(content: DocumentContent) -> str:
    """Convert document content to markdown.

    Args:
        content: DocumentContent to convert.

    Returns:
        Markdown string with appropriate formatting.
    """
    lines = []
    for para in content.paragraphs():
        md = paragraph_to_markdown(para)
        lines.append(md)
    return "\n\n".join(lines)


def write_markdown(content: str, path: Path) -> None:
    """Write markdown content to a file.

    Args:
        content: Markdown content to write.
        path: Output file path.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def write_document_as_markdown(doc_content: DocumentContent, path: Path) -> None:
    """Write document content as a markdown file.

    Args:
        doc_content: Document content to convert.
        path: Output file path.
    """
    md = content_to_markdown(doc_content)
    write_markdown(md, path)


def write_sections_as_files(
    sections: list[list[Paragraph]],
    output_dir: Path,
    *,
    default_prefix: str = "section",
) -> list[Path]:
    """Write sections as separate markdown files.

    Args:
        sections: List of sections, each containing paragraphs.
        output_dir: Directory to write files to.
        default_prefix: Prefix for numbered sections without titles.

    Returns:
        List of paths to created files.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    paths = []

    for i, section in enumerate(sections, 1):
        if not section:
            continue

        # Try to use first heading as filename
        title = None
        for para in section:
            if para.paragraph_type in (
                ParagraphType.TITLE,
                ParagraphType.HEADING1,
                ParagraphType.HEADING2,
            ):
                title = para.text
                break

        filename = f"{i:02d}-{slugify(title)}.md" if title else f"{default_prefix}-{i:02d}.md"

        content = "\n\n".join(paragraph_to_markdown(p) for p in section)
        path = output_dir / filename
        path.write_text(content, encoding="utf-8")
        paths.append(path)

    return paths
