"""DOCX text extraction with paragraph styles and break detection."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto
from pathlib import Path
from typing import Iterator
from zipfile import ZipFile

from lxml import etree


class ParagraphType(Enum):
    """Type of paragraph content."""

    BODY = auto()
    HEADING1 = auto()
    HEADING2 = auto()
    HEADING3 = auto()
    HEADING4 = auto()
    TITLE = auto()


class BreakType(Enum):
    """Type of document break."""

    PAGE = auto()
    SECTION = auto()


@dataclass
class Paragraph:
    """A paragraph with its content and metadata."""

    text: str
    paragraph_type: ParagraphType = ParagraphType.BODY
    style_name: str | None = None


@dataclass
class DocumentBreak:
    """A break in the document flow."""

    break_type: BreakType


@dataclass
class DocumentContent:
    """Structured document content."""

    elements: list[Paragraph | DocumentBreak] = field(default_factory=list)

    def paragraphs(self) -> Iterator[Paragraph]:
        """Iterate over just the paragraphs."""
        for elem in self.elements:
            if isinstance(elem, Paragraph):
                yield elem

    def sections(self) -> list[list[Paragraph]]:
        """Split content into sections based on page breaks."""
        sections: list[list[Paragraph]] = [[]]
        for elem in self.elements:
            if isinstance(elem, DocumentBreak) and elem.break_type == BreakType.PAGE:
                if sections[-1]:  # Only add new section if current has content
                    sections.append([])
            elif isinstance(elem, Paragraph):
                sections[-1].append(elem)
        return [s for s in sections if s]  # Remove empty sections


# OOXML namespaces
NAMESPACES = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "w14": "http://schemas.microsoft.com/office/word/2010/wordml",
}

# Style name to paragraph type mapping
STYLE_MAP = {
    "Title": ParagraphType.TITLE,
    "Heading1": ParagraphType.HEADING1,
    "Heading 1": ParagraphType.HEADING1,
    "Heading2": ParagraphType.HEADING2,
    "Heading 2": ParagraphType.HEADING2,
    "Heading3": ParagraphType.HEADING3,
    "Heading 3": ParagraphType.HEADING3,
    "Heading4": ParagraphType.HEADING4,
    "Heading 4": ParagraphType.HEADING4,
}


def read_docx(path: Path) -> DocumentContent:
    """Read a DOCX file and extract structured content.

    Args:
        path: Path to the DOCX file.

    Returns:
        DocumentContent with paragraphs and breaks.
    """
    with ZipFile(path) as zf:
        with zf.open("word/document.xml") as f:
            tree = etree.parse(f)

    root = tree.getroot()
    body = root.find(".//w:body", NAMESPACES)
    if body is None:
        return DocumentContent()

    content = DocumentContent()

    for elem in body:
        tag = etree.QName(elem).localname

        if tag == "p":
            para = _parse_paragraph(elem)
            if para:
                content.elements.append(para)

        elif tag == "sectPr":
            # Section properties often indicate section breaks
            content.elements.append(DocumentBreak(BreakType.SECTION))

    return content


def _parse_paragraph(elem: etree._Element) -> Paragraph | DocumentBreak | None:
    """Parse a paragraph element."""
    # Check for page breaks within the paragraph
    for br in elem.findall(".//w:br", NAMESPACES):
        br_type = br.get(f"{{{NAMESPACES['w']}}}type")
        if br_type == "page":
            return DocumentBreak(BreakType.PAGE)

    # Check for page break before in paragraph properties
    ppr = elem.find("w:pPr", NAMESPACES)
    if ppr is not None:
        page_break_before = ppr.find("w:pageBreakBefore", NAMESPACES)
        if page_break_before is not None:
            # This paragraph starts on a new page, but we still want its content
            pass

    # Extract text from all text runs
    texts = []
    for t in elem.findall(".//w:t", NAMESPACES):
        if t.text:
            texts.append(t.text)

    text = "".join(texts).strip()
    if not text:
        return None

    # Determine paragraph type from style
    style_name = None
    para_type = ParagraphType.BODY

    if ppr is not None:
        pstyle = ppr.find("w:pStyle", NAMESPACES)
        if pstyle is not None:
            style_name = pstyle.get(f"{{{NAMESPACES['w']}}}val")
            if style_name and style_name in STYLE_MAP:
                para_type = STYLE_MAP[style_name]

    return Paragraph(text=text, paragraph_type=para_type, style_name=style_name)


def read_docx_text(path: Path) -> str:
    """Read a DOCX file and return plain text content.

    Args:
        path: Path to the DOCX file.

    Returns:
        Plain text content with paragraphs separated by newlines.
    """
    content = read_docx(path)
    return "\n\n".join(p.text for p in content.paragraphs())
