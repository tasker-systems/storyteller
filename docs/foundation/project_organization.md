# Project Organization: Streams, Phases, and the Shape of the Work

## On Organizing Without Ossifying

This document maps the work ahead — not as a rigid project plan with Gantt charts and deadlines, but as a living orientation to the streams of effort, their relationships, their dependencies, and a suggested phasing that respects what must come before what. We hold this loosely. The work will teach us things that reshape the plan, and we must let it.

That said, clarity about what we are doing and in what order is not the enemy of creativity. It is its scaffold. Even improvisation benefits from knowing where the stage is.

---

## The Streams

We identify the following distinct streams of work. Each has its own character, its own rhythm, its own relationship to the others.

### Stream 1: Foundational Thinking
*Ongoing — never completes, but must reach initial maturity early*

The philosophical, analytical, and design work that shapes everything else. This includes:

- Continued evolution of the design philosophy, system architecture, character modeling, narrative graph, and open questions documents
- Research into relevant fields: narratology, actor-network theory, phenomenology of perception, game design theory, the craft of GM-ing, the craft of fiction writing
- Exploration of specific design questions as they arise from implementation — the documents are living, and implementation will surface questions we haven't imagined
- Periodic revisiting of foundational assumptions in light of what we learn from building and testing

**Depends on:** Nothing. This is the root.
**Feeds into:** Everything.
**Key outputs:** Evolving design documents, research notes, decision records.

---

### Stream 2: Software Architecture and Specification
*Early phase — substantially complete before heavy implementation, refined throughout*

The bridge between philosophy and code. Translating conceptual architecture into technical architecture:

- Data structure design for character tensors, relational webs, narrative graphs, world-state, information ledgers
- Information flow specifications: what messages move between agents, in what format, with what content, under what conditions
- API design for the boundaries between subsystems — how the engine communicates with agents, how the transport layer serves the player interface
- Crate and package structure for the Rust/Bevy codebase — separation of concerns, modularity, what belongs where
- Transport layer decisions: protobuf/gRPC vs REST, message queue strategies for agent communication, persistence strategies for world and narrative state
- Model capacity considerations: what context window sizes we assume, how we manage token budgets across agents, how we adapt to different model capabilities
- A Jobs To Be Done / persona-journey approach to our own product understanding — who are the story designers, who are the players, what are their needs, frustrations, delights

**Depends on:** Stream 1 (foundational thinking must be mature enough to specify against)
**Feeds into:** Streams 3, 4, 5, 6, 7, 8, 9, 10, 11
**Key outputs:** Technical spec documents, architecture diagrams, API contracts, data model schemas, crate structure plans.

---

### Stream 3: Core Engine Implementation
*Middle phase — begins after architecture is substantially specified*

The Bevy ECS world engine that everything runs on:

- Entity-component-system modeling for characters, locations, objects, relationships, narrative nodes
- The temporal system: advancement of narrative time (designer-triggered and cyclical), integration with event propagation
- World-state persistence: the lightweight tracking of physical changes (the cabin, the broken door, the razed town), tiered by narrative significance
- The constraint framework implementation: hard constraints (world rules), soft constraints (character capacity), perceptual constraints (what can be sensed)
- Scene management: loading scenes, tracking scene state, managing transitions between scenes and connective space
- The narrative graph as a runtime data structure: nodes, edges, weights, attractor basins, approach vectors, departure trajectories
- State serialization and persistence: saving the world between sessions, the unwind mechanic's state management

**Depends on:** Stream 2 (architecture and data structures must be specified)
**Feeds into:** Streams 5, 6, 7, 10, 11
**Key outputs:** Working Bevy application with core systems, testable in isolation.

---

### Stream 4: Agent Orchestration Layer
*Middle phase — can begin prototyping earlier with simpler infrastructure*

The custom-built coordination layer for agent communication:

- The orchestration system that manages agent instantiation, context construction, and response processing
- Context management: constructing the right prompt for each agent from the available state, respecting information boundaries
- LLM integration abstraction: a unified interface that works with API models (Claude, GPT, etc.) and local models (Ollama), with capability-aware prompting
- Message flow between agents: how Character Agent outputs reach the Reconciler, how the Reconciler's output reaches the Narrator, how the Narrator's output reaches the player
- Agent lifecycle management: instantiating Character Agents when scenes require them, releasing them when scenes conclude, maintaining continuity through the Storykeeper
- Persistent agent memory for long-running agents (Narrator, Storykeeper): how they maintain awareness across the session without exceeding context limits
- Error handling and graceful degradation: what happens when an agent produces incoherent output, when a model times out, when context limits are exceeded

**Depends on:** Stream 2 (API contracts, message schemas), partially on Stream 3 (engine state to draw from)
**Feeds into:** Streams 5, 6, 7, 10, 11
**Key outputs:** Working orchestration layer, testable with mock agents and real models.

---

### Stream 5: Agent Persona Development
*Middle-to-late phase — benefits from early experimentation, matures with use*

Developing the guidance, instruction, and personality for each core agent:

- **Narrator persona**: voice, craft, relationship to player, how it renders scene descriptions, character actions, atmospheric detail, how it signals significance without spoiling, how it handles constraint boundaries gracefully
- **Storykeeper persona**: how it reasons about narrative state, information release conditions, world-state updates, off-screen propagation, when to intervene in emergent situations and when to let them breathe
- **Reconciler persona**: how it sequences multi-character interactions, resolves conflicts, surfaces dramatic potential, manages pacing, handles physical conflict at fairy-tale granularity
- Context management strategies specific to each persona: what information each agent needs, in what form, and how to construct prompts that produce the best performance
- Identification of additional agent personas as the system matures — a World Agent for reality testing and constraint management may emerge as a distinct role, or it may remain a function of the Storykeeper
- Iterative refinement through dress rehearsal testing (Stream 10)

**Depends on:** Stream 2 (role definitions), Stream 4 (orchestration layer to run agents), Stream 1 (ongoing philosophical grounding)
**Feeds into:** Stream 10 (testing), Stream 8 (tooling for story designers interacts with agent capabilities)
**Key outputs:** Persona documents, system prompts, context construction templates, performance benchmarks.

---

### Stream 6: Character-Agent Framework
*Middle-to-late phase — one of the most complex and novel streams*

The system for modeling, instantiating, and evolving characters:

- The tensor data structure: values, traits, attributes, motivations (surface/deep/shadow), beliefs, capacities, with weights and contextual relevance
- The geological temporal model: topsoil (acute emotions), sediment (sustained patterns), bedrock (core identity), with appropriate decay rates and persistence
- The echo mechanism: detecting when current experience resonates with historical patterns, temporarily elevating old emotional states
- Context-dependent activation: determining which tensor dimensions are relevant in a given scene, constructing the activated subset for the Character Agent
- The relational web as a data structure: trust, affection, debt, power, history, projection — each edge multidimensional, each perspective potentially different
- Off-screen tensor updates: how events propagate to characters not in the current scene, with appropriate attenuation and distortion
- Pseudo-random character generation: creating plausible, textured minor characters on the fly — the shopkeeper who might remember the player's name, the traveler with a story to tell
- Character promotion: the mechanism by which a dynamically generated character gains permanence, absorbs narrative weight, potentially takes on a plot role — the gradient from background figure to significant presence
- The player-character tensor: reputation, relational impressions, information state, physical state — the world's model of the player

**Depends on:** Stream 2 (data structures), Stream 3 (engine to host the data), Stream 4 (orchestration to instantiate agents)
**Feeds into:** Stream 5 (agents draw on this framework), Stream 7 (propagation system updates tensors), Stream 10 (testing)
**Key outputs:** Character data model, tensor management system, generation and promotion mechanisms, relational web implementation.

---

### Stream 7: Information Propagation and Event System
*Middle-to-late phase — technically novel, requires research and experimentation*

The system for rippling consequences through the world:

- Graph-theoretic propagation model: how information and events travel through the relational web, with algorithmic distortion based on intermediary character properties (the Latour/ANT-informed actant model)
- Computational strategy: when propagation can be handled algorithmically (weight calculations, distance attenuation, plausibility checks) vs. when it requires agent-level reasoning (complex distortion, narrative judgment)
- Integration with the temporal system: propagation takes time, and the system must model information travel speed alongside narrative time advancement
- Event hooks: the mechanism by which story-relevant actions trigger inspection in the rules engine — when the player opens a locket, the system must determine if this is narratively meaningful and propagate consequences accordingly
- Tensor update pipelines: how propagated events translate into specific changes to character tensors, relationship weights, information states
- Narrative graph updates: how events shift narrative mass, open or close pathways, create or modify attractor basins
- World-state updates: how events change the physical, political, and social landscape, and how those changes are tracked and persisted

**Depends on:** Stream 2 (architecture), Stream 3 (engine and temporal system), Stream 6 (character tensors to update)
**Feeds into:** Stream 5 (agents respond to propagated state), Stream 10 (testing)
**Key outputs:** Propagation algorithm, event hook system, tensor update pipeline, integration with temporal system.

---

### Stream 8: Story Designer Tooling
*Later phase — follows core engine, benefits from agent maturity*

The tools that enable story designers to create content:

- World-building interface: establishing history, geography, socio-political grounding, cultural context, genre contracts (hard constraints)
- Scene authoring: defining scenes as possibility spaces — core events, required and contingent characters, approach vectors, information gates, departure trajectories, thematic register, tonal signature, narrative mass
- Character authoring: building tensor representations through guided interaction — potentially conversational ("I want a character who is loyal but secretly resentful; help me build their tensor")
- Relational web authoring: defining the connections between characters with textured, multidimensional edges
- Narrative graph visualization: seeing the gravitational landscape, the attractor basins, the pathways and convergence points
- Templates for common narrative types and tropes: noir mystery, epic fantasy, pastoral drama, political intrigue — scaffolding that the designer customizes
- Validation and consistency checking: flagging unreachable scenes, inconsistent tensors, unsatisfiable information conditions, relational contradictions
- MCP server development: enabling AI-assisted story design as a conversational, collaborative process
- Playtesting integration: the ability to enter a story at any point and test how it plays, then adjust and re-enter

**Depends on:** Streams 3, 4, 5, 6, 7 (the core system must exist for tooling to target)
**Feeds into:** Stream 10 (testing uses authored content), Stream 11 (scene development requires tooling)
**Key outputs:** Story design application or interface, MCP server, template library, validation system.

---

### Stream 9: World Engine and Constraint System
*Middle phase — overlaps with core engine but distinct in focus*

The subsystem responsible for reality testing, world rules, and persistent physical state:

- Hard constraint definition and enforcement: the genre contract, world physics, what is and is not possible
- Soft constraint adjudication: character capacity in context, the interplay of tensor state with action feasibility
- Perceptual constraint management: determining what a character can sense, infer, or intuit based on their tensor and the scene's available information
- Physical world modeling: lightweight spatial relationships (relational assertions or simple maps), persistent modifications (structures built, objects moved, environments changed)
- Political and social world modeling: power structures, alliances, territory, authority — tracked and subject to change through player action and narrative events
- Temporal aging: how the world changes with time — seasons, growth, decay, the cabin settling, the garden overgrowing
- Integration with the Narrator's craft: how constraints are communicated as narrative experience rather than system messages — the pedagogical function of boundaries
- Possible emergence of a **World Agent** as a distinct persona, or integration of these functions into the Storykeeper's responsibilities

**Depends on:** Stream 2 (architecture), Stream 3 (engine infrastructure)
**Feeds into:** Streams 4, 5, 7, 8 (all agents interact with world constraints)
**Key outputs:** Constraint framework, world-state model, spatial system, integration with agent communication.

---

### Stream 10: Testing and Dress Rehearsal
*Ongoing from early prototyping through maturity*

Testing at every level, from unit tests to full narrative rehearsals:

- Unit and integration testing: standard software testing for engine components, data structures, propagation algorithms, state management
- API contract testing: ensuring that the interfaces between subsystems behave as specified
- Agent output testing: evaluating agent responses for quality, consistency, adherence to persona, sensitivity to tensor state
- **Dress rehearsals**: full scene runs with Character Agents and Narrator, using proxy player-character interactions that represent different play styles — the curious explorer, the aggressive boundary-tester, the emotionally engaged collaborator, the disengaged minimalist, the spoilsport
- Constraint boundary testing: systematically testing hard, soft, and perceptual constraints to ensure they are communicated gracefully
- Emergence monitoring: running extended sessions to observe emergent behaviors, identifying productive emergence vs. system drift
- Model capability benchmarking: testing the same story across different model tiers to characterize the experience at each level
- The unwind mechanic testing: verifying that tensor residues persist correctly, that echoes function, that narrative coherence survives unwinding
- Performance testing: latency, throughput, token budget management, the experience of waiting for agent responses

**Depends on:** Everything above, incrementally
**Feeds into:** All streams (testing reveals what needs to change)
**Key outputs:** Test suites, dress rehearsal scripts, performance benchmarks, quality evaluations, bug reports, design revision recommendations.

---

### Stream 11: Scene and Narrative Content Development
*Later phase — requires tooling and a mature engine*

The actual creation of stories, scenes, and characters for the system:

- Development of one or more reference stories that exercise the full range of the system's capabilities — a "proof of concept" narrative designed to test depth, branching, emotional range, and mechanical soundness
- Scene development following our possibility-space model: core events, characters, approach vectors, information gates, consequences
- Player-character choices with persistent impact: designing moments where choices visibly and meaningfully change the world and relationships
- Calibration of narrative signaling: ensuring the Narrator's craft communicates significance appropriately — when to signal that a choice matters, when to let consequences surprise
- Iterative refinement through dress rehearsal (Stream 10): playing through authored content, evaluating how it lands, adjusting

**Depends on:** Streams 3, 4, 5, 6, 7, 8, 9 (the system must be substantially built)
**Feeds into:** Stream 10 (content is what gets tested), Stream 1 (authoring reveals design questions)
**Key outputs:** Playable stories, scene libraries, character libraries, design patterns for story authoring.

---

## Suggested Phasing

These streams do not proceed in strict sequence, but they do have a natural ordering. We suggest three broad phases, with the understanding that boundaries are porous and work from earlier phases continues into later ones.

### Phase 1: Foundation and Architecture
*Where we are now, and the immediate next steps*

**Primary streams:** 1 (Foundational Thinking), 2 (Architecture and Specification)
**Supporting work:** Early experiments in Streams 4 and 5 (prototyping agent communication and persona development with existing tools, before the engine exists)

The goal of this phase is to arrive at specifications solid enough to build against — data models, API contracts, crate structures, information flow diagrams — while continuing to deepen the philosophical and design foundations. We should also be experimenting with agent personas early: writing prompts for the Narrator, testing how Character Agents respond to tensor information, exploring what the Reconciler needs to do its work. These experiments don't require the engine; they can happen in conversation with models right now.

**This phase is complete when:** We have a technical specification document that a developer could build from, and we have early agent persona experiments that give us confidence the approach works.

### Phase 2: Core Systems
*Building the infrastructure*

**Primary streams:** 3 (Core Engine), 4 (Orchestration Layer), 6 (Character-Agent Framework), 7 (Propagation and Events), 9 (World Engine)
**Supporting work:** Continued Stream 1 (foundational thinking evolves with implementation), Stream 5 (agent personas refined as we test with real infrastructure), early Stream 10 (testing begins as soon as there is anything to test)

This is the heavy implementation phase. The Bevy engine, the agent orchestration, the character tensor system, the propagation model, the world engine. These can be developed somewhat in parallel — the engine and orchestration layer first, then the character and propagation systems building on them, with the world engine developing alongside.

**This phase is complete when:** We can run a simple scene — a player interacting with a Narrator, meeting a character, taking actions that propagate through the system — end to end, even if roughly.

### Phase 3: Integration, Tooling, and Content
*Making it real*

**Primary streams:** 8 (Story Designer Tooling), 10 (Testing and Dress Rehearsal at full scale), 11 (Scene and Narrative Content)
**Supporting work:** All prior streams continue to be refined based on what we learn

This phase takes the working system and makes it usable — for story designers and for players. Tooling enables content creation. Content creation tests the system. Testing reveals what needs to change. The cycle tightens.

**This phase is complete when:** A story designer who is not us can create a story, and a player who is not us can play it, and the experience is meaningful.

---

## On the Relationship Between Phases

These phases are not waterfall stages. They are tidal. Phase 1 work continues into Phase 2 and Phase 3. Phase 2 discoveries reshape Phase 1 documents. Phase 3 testing changes everything.

The most important principle is: **build the smallest thing that teaches you something, then build the next smallest thing.** A working Narrator responding to player input in a single scene, with no engine behind it, teaches us more about the experience than a month of pure specification. A character tensor fed to a Character Agent, even without the full orchestration layer, teaches us whether the representation produces the kind of performance we want.

The documents and the code must evolve together. Neither is the master of the other. The documents guide the code, and the code tests the documents.

---

## What We Do Next

Immediately:
1. Continue refining foundational documents as new ideas emerge (ongoing)
2. Begin the technical specification document — data models, information flow, API contracts (Stream 2)
3. Experiment with agent personas — write Narrator prompts, test Character Agent responses to tensor data, explore Reconciler coordination (early Stream 5)
4. Establish the Rust/Bevy project structure and begin core engine scaffolding (early Stream 3)

These four activities can proceed in parallel, and each informs the others. The spec work discovers questions that the foundational documents must address. The agent experiments reveal what data structures the tensors need. The engine scaffolding tests whether the architecture actually works in code.

We begin where we are. We build what we can see. We let the work teach us what comes next.
