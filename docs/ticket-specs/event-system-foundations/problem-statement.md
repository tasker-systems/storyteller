Thinking about events - we have three distinct classes of problem to solve:

1. Event Identification
2. Event Classification
3. Event Confidence and Calibration

In our docs/technical/event-system.md - which is outdated as of the narrator pivot docs/technical/narrator-architecture.md and docs/changelog/2026-02-07-narrator-centric-validation.md - we have described a system by which we would attempt to enable the inference of events from player input and scene information. 

There are related concerns to handle here. We need to be able to identify that an event *might* have happened. But events are not reducible to simple word tokenization. Consider even a simple scene:

```
Setting: Inside a small room, at a table, alone

Tanya is sitting at the table, looking at her phone. She is not really paying attention to it though, idly scrolling through her feed. Her face is still a bit swollen and red, with tear stains that have been hastily wiped away, but still visible.

On the table there is a cup of coffee, still warm. The cup has a small chip in it, like a tooth that has lost a bit of enamel. She almost smiles, but then tears well up again and she drops her phone to the table, buries her face in her hands, and begins to cry again in earnest.
```

In this scene, description, we have a number of possible entities and events.

Entities:

* Tanya
* Coffee cup
    - Chip
* Table
    - Implied chair
* Phone
    - Feed

Events:

* Tanya is sitting at the table
* Tanya is looking at her phone
* Tanya is not really paying attention to her phone
* Tanya is scrolling through her feed
* Tanya's face is swollen and red
* Tanya has been crying
* Tanya has a cup of coffee
* Tanya smiles
* Tanya tears well up
* Tanya drops her phone to the table
* Tanya buries her face in her hands
* Tanya begins to cry again in earnest

Some of these events are potentially narratively loaded. Some of the interactions mean that an entity should be ledgered against our permanence-and-degradation mechanics. The phone is likely something that Tanya would have consistently, and if a player referenced it and it was unknown, that would be confusing. The coffee cup having a chip in it that seems to be part of why she cries is meaningful. But we didn't express any relationships with the table, or the implied chair she would be sitting on. If our scene description had noted the sound of a muffled television playing in another room, we probably wouldn't consider it narratively loaded, but if the player asked about it, or the player went into that room and now a radio was playing, that would be confusing and lose continuity.

---

What we need is a way to identify entities, to identify events, and to surmise the necessary vs incidental relationships between them. We also need to then be able to determine combinatorial events. Tanya looking at a coffee cup is an event, but the fact of a coffee cup with a chip in it is what leads her to cry is a combinatorial event.

Simplistic rules engines are not likely to be enough to capture this. However, we will have to establish some kind of event grammar, an entity identification mechanism to ensure that our events carry referents to the entities involved in the event. The event itself does not need to track the relationships between entities, we have planned social graphs for that kind of thing, though the social graph may well need to be able to be meaningfully joined to an event.

Identifying entities and events is part of our work. But we also need event classification, which is bound up with identification - without an event grammar and a classification mechanism, saying what an event is or is not becomes very difficult. You cannot extract the bounds of a possible event without some classification mechanism and grammar.

However, our events cannot be sourced only from player input and the scene. Certain narrative and combinatorial events are very likely to both reference entities that are not in the scene, and may have context required from the prior event ledger and relational graphs to even determine if an event has occurred.

We also need to develop a mechanism for calibration and confidence scoring. In many cases there will be some degree of subjectivity about whether an event has or has not occurred. If it is something as simple as "John picks the flower" when a flower has been intentionally rendered as part of the narrative scene, that is relatively simple to identify. But if it is something like "Jane pulls the pressed flowers from the old book, and she quietly drops them into the fire, finally able to say goodbye" - well now we need to identify the pressed flowers, trace the entity relationships and social graph to attempt to understand which vertices carry emotional weight between the flowers and Jane, and whether saying goodbye is a narratively meaningful event or if it is something that should adjust Jane's self-relationship or Jane's relationship with someone living or dead on her social graph.

The final part of this is that our event grammar needs to be robust and extensible, and enable set-logic combinatorial mechanics up to N depth, and be described in a way that would actually enable story designers and authors to work with these, or at least to describe them in natural language with an MCP server of our designing, that could be translated into more discrete data structures.
