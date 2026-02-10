# Training Data Generation

## Combinatorial Generation Principle

Both ML pipelines use combinatorial generation rather than hand annotation for training data. The motivations:

**Scale**: Hundreds of annotated examples are not enough. Thousands are needed for even a small model, and hand-annotating narrative text at that scale is prohibitive.

**Reproducibility**: Every training example is generated from a deterministic seed and parameterized templates. The dataset can be regenerated exactly, audited for bias, or expanded systematically.

**Controlled variance**: The generation parameters define the variance space explicitly. We know what combinations of character profiles, relational dynamics, and scene contexts the model has seen because we designed the matrix that produced them.

The two pipelines use the same principle — combinatorial expansion of authored templates — but with different granularity suited to their different tasks.

---

## Character Prediction: Descriptor-Based Generation

The character prediction training data is generated via a three-axis combinatorial matrix. Each axis provides a set of authored templates that combine to produce diverse training scenarios.

The implementation lives at `storyteller-ml/src/matrix/` (8 submodules: `archetypes`, `dynamics`, `profiles`, `descriptors`, `combinator`, `labels`, `validation`, `export`).

### Three-Axis Matrix

**Axis 1: Character Archetypes** → tensor profiles

Narrative archetypes serve as tensor generation templates. Not Jungian essences — generative starting points that produce characteristic tensor configurations with stochastic variation.

Examples: byronic hero (high defiance, defended grief), brave mother (high protective instinct, articulate fear), wise elder (sediment-layer knowledge, defended regret). Each archetype generates a tensor profile with controlled variation across axes — no two instances are identical, but they share structural characteristics.

**Axis 2: Relational Dynamics** → edge configurations

Common relational patterns expressed as substrate dimension configurations. Mentor/student (asymmetric trust, projection), siblings (high history, complex trust), rivals with respect (moderate trust, high projection), strangers with shared grief (emergent trust, recognition). Each dynamic generates a directed edge with substrate values.

**Axis 3: Scene Profiles** → constraint and affordance sets

Common narrative situations: confrontation over betrayal (high tension, verbal affordance), vulnerable admission of grief (intimate space, emotional affordance), celebration interrupted by threat (shifting tension, social → survival transition). Each profile generates scene features with contextual constraints.

### Generation Process

For each cell in the matrix (archetype × dynamic × profile):

1. **Generate tensor**: Instantiate the archetype template with stochastic variation. Apply relational edge configuration. Set emotional state appropriate to scene entry.
2. **Generate intent**: Use an LLM to produce structured `CharacterIntent` given the full feature set. The prompt template specifies the output format exactly.
3. **Validate coherence**: Programmatic checks against the generated tensor (see Quality Validation below).
4. **Score and filter**: Assign coherence scores. Discard incoherent examples. Flag borderline cases.

### External Descriptors

The authored descriptor files (archetype templates, dynamic templates, profile templates) live as JSON files in `$STORYTELLER_DATA_PATH`. The `descriptors` module resolves the path via environment variable, with fallback to a local path for development. This keeps authored creative content out of the code repository while maintaining reproducible generation.

### Current Scale

The initial matrix targets 10-15 archetypes × 8-10 dynamics × 8-10 profiles = 640-1500 cells, with 3-5 variations per cell. The current dataset contains approximately 7,500 training examples.

---

## Event Classification: Template-Based Generation

The event classification training data uses template patterns with slot filling to generate annotated text. Unlike the character prediction pipeline (which generates float feature vectors), this pipeline produces text with character-level entity span annotations.

The implementation lives at `storyteller-ml/src/event_templates/` (5 submodules: `templates`, `vocabulary`, `expansion`, `validation`, `export`).

### Templates

Each template defines:
- A **text pattern** with named slots: `"{character} picks up {object} from {location}"`
- **Event kind labels**: which EventKinds this pattern represents (may be multi-label)
- **Entity annotations**: which slots produce entity spans, with NER categories and roles
- **Register variants**: how the pattern conjugates for player input ("I pick up...") vs narrator prose ("Sarah picked up...")

45 hand-authored templates cover all 8 classifiable EventKinds. Templates range from simple single-entity patterns to complex multi-entity compositions that produce multiple event kinds simultaneously.

### Vocabulary

The `vocabulary` module provides compiled word lists for slot filling:

- **Character names**: Drawn from the storyteller narrative universe (TFATD characters, Bramblehoof cast, generic fantasy names)
- **Objects**: Narrative-appropriate physical items ("the ancient stone", "a tarnished key")
- **Locations**: Setting references ("the riverbed", "the clearing beyond the gate")
- **Gestures, sensory details, abstracts, collectives**: Category-specific vocabularies

The vocabularies are currently compiled into the Rust source. A future iteration will externalize them to JSON files at `$STORYTELLER_DATA_PATH` (matching the character prediction descriptor pattern).

### Register-Aware Expansion

Each template expands into two register variants:

- **Player register**: First-person imperative/declarative. "I pick up the ancient stone from the riverbed."
- **Narrator register**: Third-person literary past tense. "Sarah picked up the ancient stone from the riverbed."

Verb conjugation is handled by the template system — each template's verb slot includes both forms. The register field in the output allows the training pipeline to balance or stratify by register.

### Entity Span Computation

Because the text is generated from templates with known slot positions, entity spans are computed programmatically — no manual annotation required. The expansion engine tracks character offsets as it fills slots:

1. Build the text left-to-right, recording the start position before each slot
2. Fill the slot with a vocabulary entry
3. Record the end position after the slot
4. Emit an `EntityAnnotation` with `(start, end, text, category, role)`

This produces perfect entity annotations by construction. The Python training script converts character offsets to BIO token labels using the tokenizer's `offset_mapping` — the most complex function in the training pipeline, handling subword splits, adjacent entities, and edge positions.

---

## Reproducibility

Both pipelines use deterministic generation:

### Seeding

The event classification generator accepts a random seed parameter. The production dataset was generated with seed `2026`, ensuring the exact same 8,000 examples are produced on every run.

The character prediction generator uses a separate seed for stochastic tensor variation within archetype bounds.

### Manifest Files

Each generation run produces a manifest file alongside the training data. The manifest records:

- Generation timestamp
- Seed value
- Total examples generated (before and after validation)
- Per-event-kind counts
- Generator version / parameter summary

The current event classification manifest lives at `$STORYTELLER_DATA_PATH/training-data/event_classification.manifest.json`.

### Deterministic Expansion Order

Templates are expanded in a fixed order. Vocabulary entries are selected via seeded random number generation. The combination of template order + vocabulary selection + seed produces a deterministic sequence of examples.

---

## Variance Control

### Character Prediction

Variance is controlled at three levels:

- **Cross-archetype variance**: Different archetypes produce structurally different tensor profiles (a byronic hero has fundamentally different axis configurations than a wise elder)
- **Within-archetype variance**: Stochastic variation perturbs axis values within archetype-defined bounds. Each generation of a "byronic hero" is recognizably similar but not identical
- **Cross-sample dimensions**: Genre, tone, and tension parameters add orthogonal variation. The same archetype × dynamic × profile cell produces different behavior under "folk horror" vs "space opera" genre constraints

### Event Classification

Variance is controlled through:

- **Vocabulary diversity**: Each slot draws from a vocabulary of 20-50 entries, producing varied surface forms for the same structural pattern
- **Multi-label combinations**: Templates that produce multiple event kinds create examples the model must handle with sigmoid (not softmax) output
- **Register mixing**: Approximately equal player/narrator register distribution, so the model handles both input types
- **Template diversity**: 45 templates across 8 event kinds, with varying entity counts and roles

---

## Quality Validation

### Character Prediction

Coherence validation operates on the floating-point feature space:

- **Emotional consistency**: The euclidean distance between predicted emotional shift and current state falls within variance bounds. Sudden jumps flag incoherence.
- **Relational alignment**: Actions directed at another character are consistent with edge substrate dimensions. High-trust relationships afford vulnerability; low-trust constrain disclosure.
- **Temporal stability**: Across sequences for the same character, bedrock values remain stable, sediment shifts slowly, topsoil responds to immediate context.
- **Awareness discipline**: A character at `Defended` awareness does not articulate the defended emotion directly. A character at `Structural` awareness is not conscious of it.

The validation module lives at `storyteller-ml/src/matrix/validation.rs`.

### Event Classification

Schema validation during generation:

- **Span bounds**: Entity start < end, end <= text length
- **Label vocabulary**: All event kind labels are valid members of `EVENT_KIND_LABELS`
- **BIO well-formedness**: Entity annotations produce valid BIO sequences (no orphaned I-tags)
- **Register consistency**: Player-register text uses first-person pronouns; narrator-register uses third-person past tense

The validation module lives at `storyteller-ml/src/event_templates/validation.rs`.

---

## Scaling Strategy

### Current State

| Pipeline | Examples | Method |
|----------|----------|--------|
| Character prediction | ~7,500 | Descriptor-based combinatorial matrix |
| Event classification | 8,000 | Template-based expansion (1,000 per EventKind) |

### Expansion Paths

**More descriptors/templates**: The most direct path. Each new archetype template adds ~50-150 character prediction examples. Each new event template adds ~100-200 event classification examples (vocabulary × register).

**LLM-augmented vocabulary**: Use an LLM to generate additional vocabulary entries (object names, location descriptions, character names) that maintain narrative coherence with the storyteller universe.

**Real prose annotation** (future, Phase C.6): Annotate actual narrative prose (from TFATD manuscript, play session transcripts) with event kinds and entity spans. This is the critical next step for evaluating model generalization beyond template patterns. Perfect F1 on templates validates pipeline correctness; real prose evaluation reveals true quality.

**Play session feedback**: As the system runs live sessions, player input and narrator prose become a natural source of training examples — with classifier predictions as noisy labels that can be human-corrected.
