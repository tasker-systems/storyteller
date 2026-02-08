# Classifying Events and Entities — ML Approaches

## Purpose

This document explores ML and NLP techniques for the three core classification tasks in the event system:

1. **Event classification**: Given natural language (player input or Narrator prose), identify events and classify them by `EventKind`.
2. **Entity extraction**: Identify entity mentions in text and produce `EntityRef` candidates.
3. **Relation extraction**: Given identified events and entities, infer `RelationalImplication` types — the bridge from "what happened" to "what relationships it creates."

These three tasks form a unified pipeline. They are not independent — an event creates relationships between entities, and it is the *relationship* that gives entities weight. The pipeline must be designed so that classification, extraction, and relation inference flow together, driven by the relational weight principle: **events that create no relationships produce no entity promotions, regardless of how well they are classified.**

### Why ML, Not Rules

The project's character prediction pipeline demonstrates the pattern: combinatorial training data generation → PyTorch fine-tuning → ONNX export → `ort` inference in Rust. That pipeline produces nuanced, context-sensitive predictions from a 38KB model at sub-millisecond latency.

The same pattern should apply here. Rule-based approaches for event classification and entity extraction are brittle — they rely on keyword lists and regex patterns that fail on literary language, figurative speech, and the creative variation that makes the Narrator's prose interesting. The current `classify_player_input()` is deliberately naive (hardcoded keyword patterns, confidence always 0.8) and was always intended to be replaced.

More fundamentally, the event grammar's `EventKind` taxonomy and the entity reference model's `ImplicationType` taxonomy were designed as *classification targets* — bounded vocabularies that a trained model can learn to predict. Using rules to map text to these taxonomies defeats their purpose.

### What This Document Does NOT Cover

- LLM-based classification (too slow for the turn cycle — the whole point of the architectural pivot was to remove per-turn LLM calls except the single Narrator call)
- Rule-based classification (the implementation plan's Phase B and C already cover rule-based entity resolution as a starting point; this document describes the ML path that replaces it)
- The event composition pipeline (Phase E — operates on classified atoms, not raw text)

---

## The Landscape: Academic Frameworks

### Event Extraction Paradigms

Four major frameworks define how NLP researchers think about events:

**ACE / ERE** (Automatic Content Extraction / Entities, Relations, Events): The foundational program. Defines an event as a specific occurrence with a *trigger* word and *arguments* (participants with semantic roles). ACE 2005 defines 8 event types and 33 subtypes — but these are newswire-oriented (Life, Movement, Transaction, Conflict, Contact, Justice). **Poor fit for narrative**: no EmotionalExpression, no RelationalShift, no EnvironmentalChange.

**TimeML**: Focuses on temporal annotation. Defines "event" much more broadly — any predicate denoting a situation. "Sarah walked slowly, her heart pounding" marks both "walked" and "pounding." Useful for temporal ordering within turns, but no semantic typing beyond basic aspectual categories.

**FrameNet** (Berkeley): Based on Charles Fillmore's frame semantics. Defines 1,224 *semantic frames* — schematic representations of situations with participant roles. Examples: `Self_motion` (Mover, Source, Goal), `Communication` (Communicator, Addressee, Topic), `Perception_experience` (Perceiver, Phenomenon), `Emotion_directed` (Experiencer, Stimulus). **Best fit for narrative**: FrameNet's frames map naturally to our `EventKind` taxonomy:

| EventKind | Relevant FrameNet Frames |
|---|---|
| `ActionOccurrence` | Self_motion, Manipulation, Ingestion, Creating, Destroying |
| `SpatialChange` | Motion, Placing, Removing, Arriving, Departing |
| `EmotionalExpression` | Emotion_directed, Experiencer_focused_emotion, Make_noise (sighing) |
| `SpeechAct` | Communication, Questioning, Telling, Request, Commitment |
| `InformationTransfer` | Communication, Telling, Reporting, Revealing |
| `RelationalShift` | Personal_relationship, Trust, Social_interaction_evaluation |
| `EnvironmentalChange` | Weather, Change_of_temperature, Light_movement |
| `StateAssertion` | Attributes, State_of_entity, Posture |

**MAVEN** (MAssive eVENt detection, 2020): 168 event types built from FrameNet, with 118,732 annotated event mentions across 4,480 Wikipedia documents. The largest event detection benchmark. Hierarchically organized — we could map its types to our 10 `EventKind` variants.

**Key insight**: FrameNet is the closest academic framework to our needs. Its frames capture the semantic richness of narrative actions. However, no existing framework covers our full taxonomy — we need a custom classifier trained on our domain.

### Semantic Role Labeling (SRL)

SRL answers "who did what to whom" — extracting predicate-argument structures from sentences. The standard formalism is PropBank, which labels arguments relative to predicates:

- **ARG0** (Proto-Agent) → maps to our `ParticipantRole::Actor`
- **ARG1** (Proto-Patient) → maps to our `ParticipantRole::Target`
- **ARGM-LOC** → maps to our `ParticipantRole::Location`
- **ARGM-MNR** (manner) → useful for emotional register detection
- **ARGM-DIR** (direction) → useful for spatial change events
- **ARGM-CAU** (cause) → useful for composition detection

SRL models are mature — BERT-based models achieve F1 ~86-87% on news text, ~72-78% on literary prose. The drop on literary text is real but not catastrophic: complex sentences, figurative language, and free indirect discourse degrade performance. For the storyteller's purposes, individual extraction errors wash out over time as the relational substrate accumulates across many events.

**What SRL misses**: The `Witness` role (an entity that observes but doesn't participate directly) has no PropBank equivalent. Witnesses need custom detection — likely through attention/perception verbs ("she watched," "he noticed," "they heard").

### Entity Extraction for Narrative Text

Standard Named Entity Recognition (NER) models (trained on OntoNotes/CoNLL) recognize PERSON, LOCATION, ORGANIZATION — categories designed for news text. Narrative text has fundamentally different entity types:

- Objects: "the cup," "the ancient stone," "the letter"
- Body language / gesture: "his clenched fist," "her narrowed eyes"
- Sensory phenomena: "the sound of water," "a bitter smell"
- Named non-human entities: "the Wolf," "Whisperthorn"
- Abstract concepts: "the debt," "the silence," "the path"

**BookNLP** (Bamman et al.) is the most relevant existing resource — a pipeline specifically for processing literary text: character identification, coreference resolution, quotation attribution, supersense tagging. Fine-tuned on LitBank (100 annotated fiction works).

**GLiNER** (Generalist and Lightweight NER): A BERT-based model that accepts entity type descriptions at inference time. You pass in `["object", "location", "gesture", "abstract concept"]` and it extracts matching spans — zero-shot, no fine-tuning required. This is highly relevant: entity types can change dynamically based on scene context. Available on HuggingFace as `urchade/gliner_medium-v2.1` (~210M params).

### Relation Extraction

Given two entity mentions and context, determine the semantic relationship. This maps directly to our `ImplicationType` enum (Possession, Proximity, Attention, EmotionalConnection, TrustSignal, InformationSharing, Conflict, Care, Obligation).

**The fundamental problem**: No existing relation extraction dataset (TACRED, DocRED, FewRel) has relation types that match our relational substrate. TACRED has `per:spouse`, `org:founded_by` — knowledge-graph relations designed for Wikidata population. We must train our own classifier.

**OpenIE** (Open Information Extraction) extracts untyped (subject, relation, object) triples without a predefined schema. "Sarah picked up the stone" → (Sarah, picked up, the stone). This serves as an intermediate representation: extract untyped triples, then classify each triple into an `ImplicationType`. Two-stage is more flexible than end-to-end because new relation types can be added without retraining the extraction model.

**Entity Marker approach** (Baldini Soares et al., 2019): The most effective architecture for relation classification. Insert special tokens around entity mentions: `[CLS] Sarah [E1] trusted [/E1] the [E2] Wolf [/E2] [SEP]`. Concatenate the `[E1]` and `[E2]` hidden states, then classify. Achieves F1 ~89% on standard benchmarks. Fast, ONNX-friendly.

**GLiREL**: Extension of GLiNER for zero-shot relation extraction. Define relation types in natural language and the model extracts them. Could provide initial prototyping before we have training data.

---

## Recommended Architecture: Shared Encoder, Multiple Heads

The key architectural insight: all three classification tasks (event kind, entity extraction, participant role) require the same thing — contextual understanding of natural language. A **single transformer encoder** produces contextual token representations that feed multiple lightweight classification heads.

### The Multi-Task Model

```
                    Text Input
                        │
                    Tokenizer
                  (tokenizers crate)
                        │
                        ▼
              ┌─────────────────────┐
              │  Shared Encoder     │
              │  (DistilBERT or     │
              │   DeBERTa-v3-small) │
              │  ~65-180MB ONNX     │
              └────────┬────────────┘
                       │
            ┌──────────┼──────────┐
            ▼          ▼          ▼
     ┌──────────┐ ┌────────┐ ┌────────────┐
     │  NER     │ │ Event  │ │ Participant │
     │  Head    │ │ Kind   │ │  Role Head  │
     │(per-token│ │ Head   │ │ (per-token) │
     │  BIO)    │ │([CLS]) │ │             │
     └────┬─────┘ └───┬────┘ └─────┬──────┘
          │           │            │
          ▼           ▼            ▼
     EntityRef[]   EventKind   ParticipantRole[]
```

**One forward pass, three outputs.** The encoder is the expensive part (~10-15ms); each head adds ~1-2ms. Total multi-task inference: ~12-17ms for 150 tokens.

### Why This Architecture

1. **Token budget efficiency**: One transformer forward pass instead of three. The encoder dominates latency.
2. **Shared representations help all tasks**: Entity boundaries inform event classification; event kind informs participant roles.
3. **ONNX export is clean**: PyTorch multi-output models export to ONNX with named outputs — the same pattern as the existing `CharacterPredictor` in `inference/frame.rs` which extracts `action`, `speech`, `thought`, `emotion` from a single model.
4. **Training synergy**: Multi-task learning often improves accuracy on all tasks compared to independent training.

### Model Selection

| Model | Params | ONNX Size | Inference (150 tokens, M4) | Recommendation |
|---|---|---|---|---|
| `microsoft/deberta-v3-small` | 44M | ~180MB | ~4-7ms | **Best accuracy/size tradeoff** |
| `distilbert-base-uncased` | 66M | ~260MB | ~8-12ms | Well-tested, good tooling |
| `microsoft/deberta-v3-base` | 86M | ~350MB | ~8-15ms | Higher accuracy, larger |
| `all-MiniLM-L6-v2` | 22M | ~90MB | ~2-3ms | Fastest, may sacrifice accuracy |
| `bert-base-uncased` | 110M | ~440MB | ~10-18ms | Standard, no reason to prefer |

**Recommendation**: Start with **DeBERTa-v3-small** (44M params, ~180MB ONNX). It consistently outperforms BERT and DistilBERT at equivalent sizes due to disentangled attention. Falls well within the 100ms latency budget. Fits alongside the existing character prediction model in memory.

**Monitor**: ModernBERT (December 2024) — beats DeBERTa-v3 on GLUE with less memory and 4x faster mixed-length processing. Has known ONNX export issues as of early 2025 (HuggingFace transformers issue #35545). When resolved, ModernBERT would be an excellent replacement.

---

## Task 1: Event Classification

### Approach: Multi-Label Sequence Classification

Treat each clause as a unit and assign one or more `EventKind` labels. Use **sigmoid + binary cross-entropy** (multi-label) rather than softmax (multi-class), because a single clause can express multiple event kinds: "Sarah storms out of the room" is both `SpatialChange` and `EmotionalExpression`.

```
Input sentence → Encoder → [CLS] representation (768-dim)
    → Linear layer (768 → 10) → Sigmoid → EventKind predictions
```

Apply per-class confidence thresholds (tuned on validation set, typically ~0.5) to produce binary labels.

### Two Input Registers

The classifier handles two kinds of text with different characteristics:

**Player input**: Short (1-3 clauses), imperative/declarative, conversational register. "I pick up the stone and ask Sarah about the path." Classify directly — no clause segmentation needed for most inputs.

**Narrator prose**: Longer (paragraph-level), literary register, multiple characters' actions interleaved with description. "Sarah flinched at the sound, her hands tightening on the stone. The Wolf watched from the shadows, utterly still." Requires clause segmentation as preprocessing, then per-clause classification.

Clause segmentation for prose: sentence boundaries (period, semicolon) plus coordination splitting ("and," "but," "while"). Rule-based splitting is sufficient — this is well-solved. Each clause gets classified independently.

### Why 10 Classes Is Easy

Our `EventKind` taxonomy has 10 variants. This is dramatically simpler than MAVEN (168 types) or ACE (33 subtypes). Academic benchmarks achieve 85-90% accuracy on those complex taxonomies; our 10-class problem should reach 90-95% with moderate training data. The classes are also semantically distinct — `SpeechAct` vs `SpatialChange` vs `EmotionalExpression` have very different lexical signatures.

### Confidence and the Relational Weight Principle

Event classification confidence feeds directly into relational weight computation. A high-confidence `SpeechAct` classification generates stronger `InformationSharing` and `Attention` implications than a low-confidence one. The `EventConfidence` struct carries both the confidence value and its provenance (which classifier, what evidence).

This means classification errors degrade gracefully: a misclassified event produces incorrect implications at low confidence, which contribute less to entity promotion. The relational weight accumulator smooths over individual classification noise.

---

## Task 2: Entity Extraction

### Approach: Broad Net + Focused Classification

Entity extraction uses a two-phase approach:

**Phase 1 — Broad extraction** (high recall): Identify all candidate entity mentions. Three complementary signals:
- Token-level BIO tagging via the NER head (trained on narrative entity categories: CHARACTER, OBJECT, LOCATION, GESTURE, SENSORY, ABSTRACT, COLLECTIVE)
- Known entity matching against the scene cast and tracked entity aliases (string match, always high confidence)
- Noun phrase extraction from dependency parse structure (catches everything the NER head might miss)

Merge these signals: union of NER spans, known-entity matches, and noun phrases. Deduplicate by span overlap.

**Phase 2 — Entity resolution** (high precision): For each extracted mention, determine whether it refers to a known tracked entity or is a new unresolved mention:
- Exact/fuzzy string match against scene cast → `EntityRef::Resolved(entity_id)`
- Embedding similarity for descriptive re-references → `EntityRef::Resolved(entity_id)` with moderate confidence
- No match → `EntityRef::Unresolved` with referential context (possessive, spatial, prior-mention)

### Narrative Entity Categories

Standard NER categories (PERSON, ORGANIZATION, LOCATION) are insufficient for narrative. The NER head trains on custom categories:

| Category | Examples | Standard NER Equivalent |
|---|---|---|
| CHARACTER | "Sarah," "the Wolf," "the old man" | PERSON (partial) |
| OBJECT | "the cup," "the stone," "the letter" | None |
| LOCATION | "the stream," "the doorway," "the far bank" | LOC/FAC (partial) |
| GESTURE | "his clenched fist," "her narrowed eyes" | None |
| SENSORY | "the sound of water," "a bitter smell" | None |
| ABSTRACT | "the debt," "the silence," "the path" | None |
| COLLECTIVE | "the crowd," "the wolves," "the trees" | None |

These categories serve entity extraction and initial typing. They do not determine promotion — that comes from relational weight, which comes from events.

### Coreference Resolution

Coreference (linking "she" → "Sarah" → "the girl" → "the child") is the most complex NLP task in the pipeline. Two observations constrain the approach:

1. **We have strong priors.** The scene cast is known — if Sarah, Tom, and the Wolf are in the scene, "she" has exactly one female referent. This constraint dramatically simplifies coreference compared to general-purpose systems.

2. **Cross-turn coreference is bounded.** The scene journal (Tier 2 narrator context) maintains the recent narrative. Coreference operates within this window, not across the entire story.

**Deployment approach**:
- **Immediate**: Cast-list constraint + simple anaphora rules (most recent compatible entity). This resolves ~80% of references in typical scenes.
- **Medium-term**: Transformer backbone (shared with NER/event heads) producing span embeddings + Rust-side scoring/clustering logic. The backbone exports to ONNX; the coreference scoring heads are simple linear layers implementable in Rust.
- **Full**: Word-level coreference model (wl-coref architecture). Competitive accuracy with significantly faster inference than span-based approaches.

### Scene Context as Advantage

General-purpose NER and coreference systems operate on text alone. We have additional context:
- **Scene cast**: Which entities are present, their roles, their relationships
- **Rendered space**: Where entities are spatially, what actions are available
- **Previous turns**: What entities have been mentioned recently
- **Character predictions**: What characters were predicted to do (confirms entity involvement)

This context can be injected as features alongside the text, improving accuracy beyond what any text-only model achieves. For entity linking specifically: pre-encode tracked entity descriptions as embeddings, encode mentions at inference time, match by similarity with scene-presence as a strong prior.

---

## Task 3: Relation Extraction (ImplicationType Inference)

### The Bridge: Events → Relationships → Entity Weight

This is the critical task — the one that implements the relational weight principle. Given an extracted event with its participants, infer what relational implications it creates. "Sarah picks up the stone" → `Possession(Sarah, stone)`. "Sarah watches the Wolf from behind the tree" → `Attention(Sarah, Wolf)` + `Proximity(Sarah, tree)`.

Without relation extraction, events are just classified text. With it, events produce the `RelationalImplication` instances that drive entity promotion and the relational substrate.

### Approach: Entity Marker Classification

The Entity Marker approach is the best fit — given two entities and their surrounding context, classify the relationship:

```
Input: [CLS] Sarah [E1] picked up [/E1] the [E2] stone [/E2] [SEP]
→ Encoder → Concatenate [E1] and [E2] hidden states
→ Linear classifier → ImplicationType
```

For each entity pair extracted from an event, run this classifier. The entity markers tell the model which pair to focus on, achieving F1 ~89% on standard relation classification benchmarks.

With a DistilBERT-based classifier: ~8ms per entity pair. Most events have 1-3 entity pairs, so total relation extraction per clause: ~8-24ms.

**Alternative for prototyping**: Zero-shot relation classification using NLI (Natural Language Inference) models. Frame each `ImplicationType` as a hypothesis:
- "Someone gained possession of something" → Possession
- "Someone is paying attention to someone" → Attention
- "Someone shared information with someone" → InformationSharing

An NLI model (e.g., `DeBERTa-v3-base-mnli`) tests each hypothesis against the text. No training data needed — just good natural language descriptions of each `ImplicationType`. Quality ~75-80% F1 for well-defined categories. Good for initial validation before we have training data.

### SRL as Relation Signal

Semantic Role Labeling provides a complementary signal. SRL extracts predicate-argument structures that map to participant roles (ARG0→Actor, ARG1→Target). The combination of EventKind + ParticipantRoles strongly constrains the possible ImplificationTypes:

| EventKind + Roles | Implied Relation |
|---|---|
| `SpeechAct(Actor=A, Target=B)` | Attention(A→B) + InformationSharing(A→B) |
| `ActionOccurrence(Actor=A, Target=object)` | Possession(A→object) or Attention(A→object) |
| `SpatialChange(Actor=A, Location=L)` | Proximity(A→L) |
| `EmotionalExpression(Subject=A, about=B)` | EmotionalConnection(A→B) |
| `ActionOccurrence(Actor=A, Target=B, kind=hostile)` | Conflict(A→B) |

This heuristic mapping provides a fast fallback (~1ms, rule-based) when the relation classifier is uncertain, and a validation signal when it is confident. The two approaches reinforce each other.

### Dependency Parsing for Relation Signals

Syntactic patterns from dependency parsing provide strong signals for relation types:

| Syntactic Pattern | Example | ImplicationType |
|---|---|---|
| nsubj-verb-dobj (transfer verbs) | "Sarah picked up the stone" | Possession |
| nsubj-verb-prep(beside/near) | "Sarah stood beside the Wolf" | Proximity |
| nsubj-perception_verb-dobj | "She watched the door" | Attention |
| nsubj-speech_verb-dobj | "He told her everything" | InformationSharing |
| nsubj-conflict_verb-dobj | "They fought the creature" | Conflict |
| poss (possessive) | "Sarah's stone" | Possession |
| nsubj-emotion_verb-prep(about) | "She feared the dark" | EmotionalConnection |

Dependency parsing is robust on literary text (~88-92% UAS vs ~95% on news) because it operates on syntax rather than semantics. The main failure mode — attachment ambiguity in complex sentences — affects specific patterns but not the overall extraction rate.

---

## Training Data Generation

### The Combinatorial Matrix Approach

The project has a proven pattern for synthetic training data (character prediction pipeline in `storyteller-ml/src/matrix/`). The same approach applies to NLP tasks, generating labeled examples for event classification, entity annotation, and relation classification.

### Event Classification Training Data

Templates per `EventKind`, parameterized by participants, objects, locations, modifiers:

```
ActionOccurrence templates:
  - "I {verb} the {object}"
  - "{character} {verb}s {adverb} toward the {object}"
  - "I try to {verb} {object_with_article}"

SpeechAct templates:
  - "I ask {character} about {topic}"
  - "I tell {character} that {statement}"
  - "{character} whispers something to {other_character}"

SpatialChange templates:
  - "I walk to the {location}"
  - "I leave the {location}"
  - "{character} enters from the {direction}"

EmotionalExpression templates:
  - "{character}'s voice trembles with {emotion}"
  - "Tears well up in {character}'s eyes"
  - "A {emotion_adjective} look crosses {character}'s face"
```

**Combinatorial expansion**: Verbs per type (~20-50) × characters (~10-20) × objects/locations (~30-50) × modifiers (~20-30) × Plutchik emotional primaries (8) = thousands of examples per `EventKind` from a modest template set.

**LLM augmentation** (same as character prediction): Generate scenario skeletons from the matrix, use Ollama to produce naturalistic literary variations, validate against schema, human review a sample.

**Narrator prose examples**: Take existing narrative text (from Bramblehoof workshop material in `storyteller-data`), use an LLM to annotate clauses with `EventKind` labels, human review and correction. Produces training data matching the actual literary register.

### Entity Extraction Training Data

Entity type × referential pattern × narrative context:

**Entity types**: Named characters, role descriptions ("the girl"), objects, scene locations, body language/gesture, sensory phenomena, abstract concepts, collective entities.

**Referential patterns**: First mention (indefinite), subsequent mention (definite), pronoun, descriptive re-reference ("the chipped vessel"), possessive ("its rim"), metonymic ("the house fell silent").

**Narrative contexts**: Action sequences, dialogue with attribution, interior monologue, description passages, transitional/scene-setting.

Since we control generation, we know where entities are — programmatic annotation of spans. Output in BIO/IOB2 format for token classification training.

### Relation Classification Training Data

TACRED-format examples (sentence + subject span + object span + relation label):

```json
{
  "token": ["Sarah", "trusted", "the", "Wolf"],
  "subj_start": 0, "subj_end": 0, "subj_type": "CHARACTER",
  "obj_start": 3, "obj_end": 3, "obj_type": "CHARACTER",
  "relation": "TrustSignal"
}
```

Templates per `ImplicationType` with combinatorial expansion:
- Entity pair configurations: character-character, character-object, character-location, character-abstract
- Tense/voice variations: active, passive, past, present
- Hedging: "Sarah seemed to trust" → TrustSignal(uncertain)
- Negation: "Sarah did NOT trust" → TrustSignal(negative)

**LLM augmentation** produces literary-quality variations:
```
Template: "{A} trusted {T}"
Variations:
  - "Something in the Wolf's eyes made Sarah lower her guard"
  - "She found herself believing the creature, despite everything"
  - "Trust — or something close enough — had settled between them"
```

### Training Data Volume Requirements

For fine-tuning a pre-trained encoder on bounded taxonomies:

| Task | Classes | Examples/Class | Total | Expected Accuracy |
|---|---|---|---|---|
| EventKind classification | 10 | 500-1,000 | 5,000-10,000 | 90-94% |
| Entity extraction (NER) | 7 categories | 500-1,000 | 3,500-7,000 | 88-92% |
| Relation classification | 9 types | 500-2,000 | 4,500-18,000 | 85-90% |

The combinatorial template approach generates these volumes easily. The character prediction pipeline already produces ~3,600 training samples per matrix run from the combinatorial expansion in `storyteller-ml/src/matrix/combinator.rs`.

---

## Rust Inference Pipeline

### The Stack: `ort` + `tokenizers`

Both dependencies are production-grade and fit the existing project:

**`ort` v2.0.0-rc.11** (already in the dependency graph): ONNX Runtime bindings. The `CharacterPredictor` in `inference/frame.rs` demonstrates the pattern — `Session` wrapped in `Mutex`, dedicated rayon thread pool, named output extraction. NLP models use the identical API; the only difference is input format (integer token IDs vs. float feature vectors).

**`tokenizers` v0.22** (new dependency): HuggingFace's canonical Rust tokenizer. This IS the reference implementation — the Python `tokenizers` package is a binding to this Rust code. Supports WordPiece (BERT), BPE (RoBERTa/GPT), Unigram (SentencePiece/T5). Loads `tokenizer.json` files exported alongside ONNX models.

Key integration detail: `get_offsets()` maps tokens back to character positions in the original text, enabling entity span extraction from token-level predictions. `get_word_ids()` maps subword tokens to original word indices, handling WordPiece splitting ("playing" → `["play", "##ing"]`).

### Multi-Task Inference in Rust

The multi-task model exports as a single ONNX file with multiple named outputs — matching the `CharacterPredictor` pattern:

```rust
// Single forward pass
let outputs = session.run(ort::inputs![
    "input_ids" => ids_tensor,
    "attention_mask" => mask_tensor,
    "token_type_ids" => type_tensor,
])?;

// Extract all task outputs
let ner_logits = outputs["ner_logits"].try_extract_array::<f32>()?;      // [1, seq, num_ner_labels]
let event_logits = outputs["event_logits"].try_extract_array::<f32>()?;   // [1, num_event_kinds]
let role_logits = outputs["role_logits"].try_extract_array::<f32>()?;     // [1, seq, num_roles]
```

Post-processing (argmax per token for NER/roles, sigmoid threshold for event kinds, span assembly) is lightweight Rust — ~2-5ms.

### Latency Budget

Target: <100ms total for the NLP pipeline on player input (1-3 clauses, ~50-150 tokens).

| Component | Latency (M4 est.) |
|---|---|
| Tokenization (`tokenizers` crate) | ~1ms |
| Multi-task encoder + heads (DeBERTa-v3-small ONNX) | ~5-10ms |
| Post-processing (argmax, span assembly, confidence) | ~2-5ms |
| Relation classification (entity marker, per pair) | ~5-10ms per pair |
| **Total (player input, 1-3 entity pairs)** | **~15-30ms** |

For Narrator prose (longer, 5-15 clauses): clause segmentation + per-clause processing. Can run asynchronously (not in critical path) at ~100-200ms total. Well within the turn cycle budget.

### Python Training Pipeline

Follows the established pattern in `training/`:

1. **Generate training data** (Python, combinatorial templates + LLM augmentation) → JSONL output
2. **Fine-tune model** (PyTorch + HuggingFace Transformers) — standard multi-task training loop with shared encoder
3. **Export to ONNX** via HuggingFace Optimum: `optimum-cli export onnx --model path/to/model --optimize O2`
4. **Inference in Rust** via `ort` + `tokenizers`

This is the exact same shape as the character prediction pipeline (Steps 1-5 of Phase 0).

---

## Zero-Shot Bootstrapping Strategy

Before training data exists, zero-shot models enable immediate prototyping:

### GLiNER for Entity Extraction

Pass narrative entity type descriptions at inference time:

```python
entity_types = ["character", "object", "location", "gesture", "sensory phenomenon", "abstract concept"]
predictions = gliner_model.predict(text, entity_types)
```

~210M params, ~15-40ms inference. ONNX export supported. Quality is lower than fine-tuned models but sufficient for validating the entity extraction pipeline before investing in training data generation.

### NLI for Relation Classification

Frame each `ImplicationType` as an NLI hypothesis and test against the text:

```
Premise: "Sarah picked up the ancient stone from the riverbed"
Hypothesis: "Someone gained possession of something"
→ NLI model → entailment probability (high for Possession)
```

Test all 9 `ImplicationType` hypotheses in one batched call. ~20ms per hypothesis with DeBERTa-v3-base-mnli. Quality ~75-80% for well-defined categories.

### Phased Transition

```
Phase 0 (immediate):  Zero-shot (GLiNER + NLI) → validate pipeline, measure quality
Phase 1 (weeks):      Generate training data via combinatorial templates
Phase 2 (weeks):      Fine-tune multi-task model, replace zero-shot components
Phase 3 (ongoing):    Accumulate human corrections, periodic retraining
```

Zero-shot quality establishes a baseline. Fine-tuned quality exceeds it. The pipeline architecture stays the same — only the models change.

---

## `[INVESTIGATION NEEDED]` Open Questions

### Clause Segmentation Quality

Does rule-based clause splitting (sentence boundaries + coordination) lose too much context for Narrator prose classification? Literary sentences can contain subordinate clauses, parenthetical constructions, and inversions that split poorly.

**Starting point**: Rule-based splitting. **Measure**: Classification accuracy on manually annotated prose passages. **Fallback**: If clause-level classification loses too much context, add paragraph-level features (encode the full paragraph, use per-clause attention windows).

### Shared Encoder vs. Separate Models

Multi-task models share an encoder but the tasks might compete during training (NER accuracy might drop when event classification is added). Single-task models avoid this but triple the inference cost.

**Starting point**: Separate models (Approach B from the Rust inference research — validates each task independently). **Transition**: Consolidate into multi-task model when task definitions stabilize.

### Coreference Scope

Per-turn coreference (just the current input) vs. per-scene (accumulated context across the scene journal). Per-scene is more accurate but requires managing a growing context window.

**Starting point**: Per-turn with cast-list constraint (resolves ~80% of references). **Extension**: Per-scene operating over the Tier 2 scene journal — uses the same text window the Narrator already sees.

### Relation Extraction: Pairwise vs. Joint

The entity marker approach classifies one entity pair at a time. Joint models (SpERT, UniRel) extract entities and relations simultaneously. Joint models have lower error propagation but are harder to export to ONNX.

**Starting point**: Pairwise (simpler, ONNX-friendly). **Monitor**: GLiREL for zero-shot joint extraction as a prototyping tool.

### Training Data Register Balance

Player input (conversational, imperative) and Narrator prose (literary, past tense) are different registers. A model trained only on one may underperform on the other.

**Solution**: Training data generation explicitly produces both registers. Templates generate both "I pick up the stone" (player) and "She reached down and lifted the stone from the cold earth" (narrator). The combinatorial matrix includes register as an axis.

---

## Relationship to the Implementation Plan

This document informs revisions to the implementation plan's Phases C, D, and the overall workflow orientation:

**Phase C (Classifier Enhancement)** should be oriented toward building the ML classification pipeline (training data generation, model fine-tuning, ONNX export) rather than extending the rule-based classifier. The existing `classify_player_input()` continues to serve as the immediate baseline while the ML pipeline is developed alongside it.

**Phase D (Turn-Unit Extraction)** should use the same ML classifier for Narrator prose, not a separate prose-specific system. The multi-task model handles both registers. Clause segmentation is preprocessing, not a different classifier.

**The relational weight principle as workflow driver**: The implementation plan should make explicit that the pipeline's purpose is not "classify events" as an end in itself, but "discover relationships through event classification." This means:
- Entity extraction is not just NER — it's finding participants in relationships
- Event classification is not just taxonomy — it's identifying what kind of relationship an event creates
- Relation extraction is the culminating step — the one that produces the `RelationalImplication` instances that drive entity promotion

The workflow flows through the relational weight principle:

```
Text → Events (classification) → Participants (extraction) → Relationships (inference)
                                                                     │
                                                                     ▼
                                                            Entity promotion
                                                            (relational weight
                                                             accumulation)
```

Every step serves the next. Classification without extraction is incomplete. Extraction without relation inference is inert. Relation inference without weight accumulation changes nothing in the system.
