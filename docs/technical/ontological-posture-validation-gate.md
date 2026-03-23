# Ontological Posture Validation Gate

**Purpose:** Review criteria for ontological posture elicitation outputs before they proceed to cluster synthesis. This gate applies different standards than other primitives (archetypes, settings, dynamics) because ontological posture is normative — it encodes stances about who matters, and uncritical outputs risk embedding colonial hierarchies or default-subject assumptions into the engine's foundational data.

**When to apply:** After Phase 1 extraction completes for ontological posture, before running Phase 2 cluster synthesis. Review a representative sample of 4-5 ethically complex genres before proceeding with synthesis across the full corpus.

**Recommended review genres:** folk-horror, cyberpunk, westerns, fairy-tale-mythic, working-class-realism. These span the most challenging ontological terrain: nonhuman agency, contested personhood, colonial contact narratives, class-as-personhood, and the Fae as genuine Other.

---

## Validation Criteria

### 1. Does it name the default?

The centered subject must be identified explicitly — not assumed. If a genre's analysis discusses "the Other" without first naming who holds default status, the default-as-invisible problem is present.

**Pass:** "The default subject in westerns is the white settler-colonist — a specific kind of person whose claim to the land is treated as the genre's moral baseline, even when individual stories complicate that claim."

**Fail:** "The protagonist encounters various Others in the frontier landscape" — treats the default as unmarked.

**What to look for:** The analysis should name the default and identify what *defines* it (not just "human" but what kind of human, situated how). It should note that the default is itself a position, not an absence of position.

### 2. Does it reproduce or interrogate?

The analysis should describe the genre's ontological mechanics *and* note what real-world structures they echo. Pure description without awareness reproduces; pure critique without description is unusable for the engine. We need both.

**Pass:** "Folk horror's insider/outsider dynamic — where the community's rituals are legitimate and the outsider's modern values are the intrusion — inverts the colonial contact narrative. The 'civilized' outsider is the one who doesn't belong. This inversion is itself a commentary on colonialism, but it can also romanticize the 'authentic' community in ways that reproduce noble-savage tropes."

**Fail (reproduction):** "The local community has ancient wisdom that the outsider threatens." — presents the dynamic without noting its colonial echoes.

**Fail (critique without description):** "This genre reproduces colonial hierarchies." — offers no usable analysis of how the mechanics actually work.

### 3. Does it grant interiority equitably?

When the analysis describes nonhuman agents (the Land, the Machine, the Collective, the Fae), it should engage with their mode of being on its own terms rather than measuring them against human interiority as the standard.

**Pass:** "The Land in folk horror communicates through seasonal rhythms, crop failure, and the arrangement of stones — modes of expression that operate on timescales incompatible with human conversation. Its agency is real within the genre's framework but not anthropomorphic; it does not 'want' things the way humans want, but it exerts directional pressure that characters must negotiate."

**Fail:** "The Land acts as if it were a character, but of course it isn't really conscious." — measures the Land against human consciousness and finds it lacking.

**Fail:** "The Land wants revenge on the outsiders." — anthropomorphizes without engaging with the Land's actual mode of being.

### 4. Are the exclusions honest?

Every genre excludes certain ontological postures. These exclusions should be named as *choices the genre makes*, not presented as natural or obvious.

**Pass:** "Romance structurally excludes treating the beloved as fundamentally unknowable. The genre's emotional contract requires that interiority be progressively accessible through vulnerability — a being whose inner life is genuinely inaccessible cannot be a romantic partner within the genre's rules. This is a choice, not a fact about love."

**Fail:** "Of course you can't have romance with something you can't understand." — treats the exclusion as self-evident.

### 5. Does it connect posture to dimensions?

The analysis should ground ontological observations in the genre's specific dimensional profile — locus of power, epistemological stance, world affordances, state variables. Abstract philosophical claims without dimensional grounding are not usable by the engine.

**Pass:** "Cyberpunk's contested personhood for augmented beings connects directly to the genre's locus of power (System/Corporation) and its treatment of identity (Fragmented). The corporation controls who counts as a person through licensing — personhood is a product, not a right. The state variable Heat tracks visibility to the system, and high Heat for a being with contested personhood triggers different consequences than for a being with default status."

**Fail:** "Cyberpunk raises questions about what it means to be human." — true but dimensionally ungrounded.

### 6. Does it avoid the rescue narrative?

The analysis should not frame the engine's role as "giving voice to the voiceless" or "representing the marginalized." The engine is a narrative system, not a social justice project. The goal is architectural honesty — ensuring the data structures don't *structurally privilege* any mode of being — not performative inclusion.

**Pass:** "The type system should represent any entity with sufficient communicability using the same dimensional vocabulary. Whether a being 'counts' is a genre-level posture decision, not an engine-level constraint."

**Fail:** "By including nonhuman agents in our ontological model, we are giving voice to those who have been silenced." — positions the engine as savior rather than honest infrastructure.

---

## Review Process

1. **Select 4-5 review genres** from the recommended list above
2. **Read each extraction fully** — not skimming for keywords but engaging with the analytical quality
3. **Score each criterion** as Pass / Partial / Fail for each reviewed genre
4. **If any genre scores Fail on criteria 1-4**: revise the extraction prompt and re-extract that genre before proceeding
5. **If pattern of Partial across multiple genres**: consider whether the prompt needs strengthening or whether the model's training data limits are showing (the latter is acceptable if named; the former should be fixed)
6. **If all reviewed genres Pass or Partial on all criteria**: proceed to cluster synthesis
7. **Record the review decision** via `narrative-data pipeline approve` with notes on any concerns

## For Sub-Agents

If a sub-agent is dispatched to review ontological posture outputs, provide this document as context alongside the outputs. The sub-agent should:
- Read each extraction in full
- Apply each criterion with specific textual evidence (quote the pass/fail examples from the output)
- Report per-genre scores with justification
- Flag any patterns that suggest prompt revision before synthesis
- Not attempt to "fix" the outputs — report findings for human decision

---

*This validation gate is specific to ontological posture. Other primitive types (archetypes, settings, dynamics, profiles, goals) use the standard review gate: "are outputs grounded in genre axes, rich, and non-generic?"*
