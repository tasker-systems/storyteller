# storyteller

A world-building and storytelling engine — a multi-agent system where distinct AI agents collaborate to create interactive narrative experiences.

## What This Is

Storyteller is a system where a human player navigates a richly authored world through natural language, guided by AI agents that each hold partial knowledge of the story. Think theater company, not agent swarm: a Narrator who speaks to the player, a Storykeeper who guards the mystery, Character Agents who inhabit roles without knowing they're in a story, a Reconciler who sequences multi-character scenes, and a World Agent who enforces the rules of reality.

The system is built on several key ideas:
- **Narrative gravity** — stories are not branching trees but gravitational landscapes where pivotal scenes exert pull
- **Character as tensor** — characters are represented as multidimensional, context-sensitive models with geological temporal layers, not flat stat sheets
- **Imperfect information** — no agent has complete knowledge; richness emerges from the interplay of partial perspectives
- **Coherent worlds** — material conditions and social forms mutually produce each other; worlds feel real when their elements imply one another

## Status

**Pre-alpha.** Extensive design documentation exists. Rust/Bevy implementation has not yet begun. A Python `doc-tools` package for extracting creative content from Scrivener and DOCX is functional.

## Documentation

All design documentation lives in [`docs/`](docs/):

| Section | Contents |
|---|---|
| [`docs/foundation/`](docs/foundation/) | Design philosophy, system architecture, character modeling, narrative graph, world design, project organization, open questions |
| [`docs/technical/`](docs/technical/) | Tensor case studies, narrative graph mapping, agent message catalog, relational web specification |
| [`docs/storybook/`](docs/storybook/) | Source creative works — analytical references and workshop material |

See [`docs/README.md`](docs/README.md) for a full guide.

## Project Structure

```
storyteller/
├── docs/
│   ├── foundation/          # Design philosophy and principles (9 documents)
│   ├── technical/           # Specifications and case studies (5 documents)
│   └── storybook/           # Source creative material
│       ├── extracted/       # Content extracted via doc-tools
│       │   ├── the-fair-and-the-dead/   # Analytical reference
│       │   └── vretil/                  # Analytical reference
│       └── bramblehoof/     # Workshop material
├── doc-tools/               # Python package for content extraction
│   ├── pyproject.toml
│   └── src/doc_tools/
├── CLAUDE.md                # Developer guidance (Claude Code)
├── AGENTS.md                # Developer guidance (Warp)
└── README.md                # This file
```

## Technical Direction

- **Rust + Bevy ECS** for the core engine — entities as characters/locations/objects, components as traits/states/relationships, systems as agents and the narrative engine
- **Custom orchestration layer** for agent communication — the information flow is too specific to the design for third-party frameworks
- **LLM integration** supporting both API models (Claude, GPT) and local models (Ollama)

See [`docs/foundation/project_organization.md`](docs/foundation/project_organization.md) for the full work stream breakdown.

## doc-tools (Python)

A functional package for extracting creative content into structured markdown:

```bash
cd doc-tools
uv sync --dev                       # Install dependencies
uv run extract-scrivener <path>     # Extract Scrivener project
uv run convert-docx <path>          # Convert DOCX to structured markdown
uv run pytest                       # Run tests
uv run ruff check .                 # Lint
```

Requires Python >= 3.11. See `doc-tools/pyproject.toml` for configuration.

## License

See [LICENSE](LICENSE).
