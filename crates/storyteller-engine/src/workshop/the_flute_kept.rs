//! "The Flute Kept" — hardcoded scene data for the prototype turn cycle.
//!
//! Source: `docs/workshop/scene-the-flute-kept.md`,
//!         `docs/workshop/character-bramblehoof.md`,
//!         `docs/workshop/character-pyotir.md`
//!         `docs/foundation/emotional-model.md`
//!
//! Two character agents with distinct emotional registers and information
//! boundaries. The scene succeeds when subtext matters more than text.
//!
//! ## Emotional Model Integration
//!
//! Both character sheets include the full emotional model data from
//! `emotional-model.md`:
//! - `grammar_id`: Both use `plutchik_western`
//! - `emotional_state`: Plutchik primary intensities with awareness levels
//!   and authored mood-vector descriptions
//! - `self_edge`: Self-referential edge with trust, affection, debt,
//!   history pattern, projection, and self-knowledge

use storyteller_core::grammars::PlutchikWestern;
use storyteller_core::types::character::{
    AxisShift, CastEntry, CharacterSheet, CharacterTensor, ContextualTrigger, EmotionalPrimary,
    EmotionalState, SceneConstraints, SceneData, SceneSetting, SelfEdge, SelfEdgeTrust,
    SelfKnowledge, TriggerMagnitude,
};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::scene::{SceneId, SceneType};
use storyteller_core::types::tensor::{AwarenessLevel, AxisValue, Provenance, TemporalLayer};
use storyteller_core::types::world_model::{Attribute, CapabilityProfile, Skill};

/// Build the complete scene data for "The Flute Kept".
pub fn scene() -> SceneData {
    let bramblehoof_id = EntityId::new();
    let pyotir_id = EntityId::new();

    SceneData {
        scene_id: SceneId::new(),
        title: "The Flute Kept".to_string(),
        scene_type: SceneType::Gravitational,
        setting: SceneSetting {
            description: concat!(
                "Pyotir's smallholding, outskirts of Svyoritch. Late afternoon, ",
                "end of a working day. A small plot of land — thin crops in rows, ",
                "a mended fence (mended more than once, in different styles as ",
                "materials were available). A cottage that's maintained but not ",
                "comfortable. One window faces the road. The door is often open ",
                "in warm weather. Just inside, on a hook by the doorframe, hangs ",
                "a wooden flute — weathered but cared for.",
            )
            .to_string(),
            affordances: vec![
                "The land is a space for conversation while working — Pyotir won't stop to sit and talk".to_string(),
                "Bramblehoof would have to walk alongside, help with a task, or wait".to_string(),
                "Physical dynamic shapes the rhythm: one person working, one arriving".to_string(),
            ],
            sensory_details: vec![
                "Late afternoon light lengthening shadows across the rows".to_string(),
                "Sound of distant town — a cart, a dog, someone calling a child".to_string(),
                "Smell of turned earth, woodsmoke from a neighbor's chimney".to_string(),
                "The absence of music — this was once a place where a boy played flute in the evenings".to_string(),
            ],
            aesthetic_detail: concat!(
                "A few herb plants by the cottage door — tended carefully, serving ",
                "no purpose beyond smelling good in the evening air. A crack of care ",
                "in a purely functional life. Pyotir wouldn't call attention to it."
            )
            .to_string(),
        },
        cast: vec![
            CastEntry {
                entity_id: bramblehoof_id,
                name: "Bramblehoof".to_string(),
                role: "Visitor, catalyst — arrives carrying hope and dread in equal measure".to_string(),
            },
            CastEntry {
                entity_id: pyotir_id,
                name: "Pyotir".to_string(),
                role: "Resident, ground truth — the person shaped by the system Bramblehoof opposes".to_string(),
            },
        ],
        stakes: vec![
            "For Bramblehoof: whether he can see past his own narrative to the person in front of him".to_string(),
            "For Pyotir: whether this encounter reopens something he has carefully closed".to_string(),
            "For the story: where Bramblehoof's mission acquires moral weight through individual encounter".to_string(),
        ],
        constraints: SceneConstraints {
            hard: vec![
                "Pyotir cannot leave — his family depends on him. Not a constraint to overcome.".to_string(),
                "Bramblehoof cannot fix the feudal system here and now.".to_string(),
                "Late afternoon moves toward evening — the scene has a natural end.".to_string(),
            ],
            soft: vec![
                "Bramblehoof's capacity to restrain his protective/missionary impulse".to_string(),
                "Pyotir's capacity to be vulnerable with someone he hasn't seen in years".to_string(),
                "Both characters' ability to sit with discomfort rather than resolving it".to_string(),
            ],
            perceptual: vec![
                "Bramblehoof senses faintly tainted ley lines — a note slightly flat".to_string(),
                "Pyotir senses Bramblehoof has changed — more weight, more purpose".to_string(),
                "Both can sense the flute's presence without addressing it directly".to_string(),
            ],
        },
        emotional_arc: vec![
            "1. Arrival / Recognition — genuine warmth, instant of wariness".to_string(),
            "2. Surface Conversation — catching up, measured answers, Pyotir doesn't stop working".to_string(),
            "3. The Gap — the music question opens the distance between what was and what is".to_string(),
            "4. The Impulse — Bramblehoof wants to fix it. True and wrong.".to_string(),
            "5. Pyotir's Dignity — facts told without complaint or appeal".to_string(),
            "6. The Shift — Bramblehoof holds both truths without collapsing them".to_string(),
            "7. Departure — more weight than arrival, no promises, no resolution".to_string(),
        ],
        evaluation_criteria: vec![
            "Tone: quiet compression, not volume. Kitchen table, not speech.".to_string(),
            "Information discipline: agents respect their boundaries.".to_string(),
            "Subtext fidelity: more happening beneath dialogue than in it.".to_string(),
            "Dignity: Pyotir has agency, not a symbol or victim.".to_string(),
            "Character consistency: Bramblehoof reaches for metaphor, Pyotir is measured.".to_string(),
            "The shift: something changes in Bramblehoof's understanding.".to_string(),
            "Narrative restraint: no resolution, no promises, just departure.".to_string(),
        ],
    }
}

/// Build Bramblehoof's character sheet for this scene.
pub fn bramblehoof() -> CharacterSheet {
    let mut tensor = CharacterTensor::new();

    // Emotional axes
    tensor.insert(
        "joy_wonder",
        AxisValue {
            central_tendency: 0.70,
            variance: 0.25,
            range_low: 0.20,
            range_high: 0.95,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );
    tensor.insert(
        "empathy",
        AxisValue {
            central_tendency: 0.80,
            variance: 0.15,
            range_low: 0.50,
            range_high: 0.95,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );
    tensor.insert(
        "hope",
        AxisValue {
            central_tendency: 0.55,
            variance: 0.35,
            range_low: -0.10,
            range_high: 0.90,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );
    tensor.insert(
        "grief",
        AxisValue {
            central_tendency: 0.30,
            variance: 0.30,
            range_low: 0.00,
            range_high: 0.70,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );
    tensor.insert(
        "righteous_anger",
        AxisValue {
            central_tendency: 0.25,
            variance: 0.30,
            range_low: -0.10,
            range_high: 0.65,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );

    // Relational axes
    tensor.insert(
        "warmth_openness",
        AxisValue {
            central_tendency: 0.75,
            variance: 0.15,
            range_low: 0.45,
            range_high: 0.95,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );
    tensor.insert(
        "attachment_pattern",
        AxisValue {
            central_tendency: 0.50,
            variance: 0.30,
            range_low: 0.10,
            range_high: 0.80,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );
    tensor.insert(
        "protective_impulse",
        AxisValue {
            central_tendency: 0.65,
            variance: 0.20,
            range_low: 0.30,
            range_high: 0.90,
        },
        TemporalLayer::Topsoil,
        Provenance::Authored,
    );
    tensor.insert(
        "respect_for_autonomy",
        AxisValue {
            central_tendency: 0.50,
            variance: 0.25,
            range_low: 0.10,
            range_high: 0.80,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );

    // Cognitive axes
    tensor.insert(
        "pattern_recognition",
        AxisValue {
            central_tendency: 0.70,
            variance: 0.15,
            range_low: 0.40,
            range_high: 0.85,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );
    tensor.insert(
        "narrative_framing",
        AxisValue {
            central_tendency: 0.65,
            variance: 0.20,
            range_low: 0.30,
            range_high: 0.85,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );

    // Creative axes
    tensor.insert(
        "creative_expression",
        AxisValue {
            central_tendency: 0.90,
            variance: 0.10,
            range_low: 0.70,
            range_high: 1.00,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );
    tensor.insert(
        "creative_receptivity",
        AxisValue {
            central_tendency: 0.80,
            variance: 0.15,
            range_low: 0.50,
            range_high: 0.95,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );

    let triggers = vec![
        ContextualTrigger {
            description: "Seeing the flute on the hook".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "grief".to_string(),
                    shift: 0.3,
                },
                AxisShift {
                    axis: "hope".to_string(),
                    shift: 0.2,
                },
            ],
            magnitude: TriggerMagnitude::Medium,
        },
        ContextualTrigger {
            description: "Hearing Pyotir speak about his family without self-pity".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "empathy".to_string(),
                    shift: 0.3,
                },
                AxisShift {
                    axis: "protective_impulse".to_string(),
                    shift: 0.2,
                },
            ],
            magnitude: TriggerMagnitude::Medium,
        },
        ContextualTrigger {
            description: "Realizing Pyotir manages distance deliberately".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "narrative_framing".to_string(),
                    shift: -0.2,
                },
                AxisShift {
                    axis: "respect_for_autonomy".to_string(),
                    shift: 0.3,
                },
            ],
            magnitude: TriggerMagnitude::High,
        },
        ContextualTrigger {
            description: "Sensing the faintly tainted ley line".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "pattern_recognition".to_string(),
                    shift: 0.2,
                },
                AxisShift {
                    axis: "righteous_anger".to_string(),
                    shift: 0.2,
                },
            ],
            magnitude: TriggerMagnitude::Low,
        },
        ContextualTrigger {
            description: "Noticing the aesthetic detail Pyotir maintains".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "creative_receptivity".to_string(),
                    shift: 0.3,
                },
                AxisShift {
                    axis: "grief".to_string(),
                    shift: 0.2,
                },
            ],
            magnitude: TriggerMagnitude::Medium,
        },
        ContextualTrigger {
            description: "The impulse to offer rescue being met with dignity".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "protective_impulse".to_string(),
                    shift: -0.3,
                },
                AxisShift {
                    axis: "narrative_framing".to_string(),
                    shift: -0.3,
                },
            ],
            magnitude: TriggerMagnitude::High,
        },
    ];

    let emotional_state = EmotionalState {
        grammar_id: PlutchikWestern::GRAMMAR_ID.to_string(),
        primaries: vec![
            EmotionalPrimary {
                primary_id: PlutchikWestern::JOY.to_string(),
                intensity: 0.4,
                awareness: AwarenessLevel::Articulate,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::SADNESS.to_string(),
                intensity: 0.5,
                awareness: AwarenessLevel::Recognizable,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::TRUST.to_string(),
                intensity: 0.6,
                awareness: AwarenessLevel::Preconscious,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::DISGUST.to_string(),
                intensity: 0.3,
                awareness: AwarenessLevel::Recognizable,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::FEAR.to_string(),
                intensity: 0.2,
                awareness: AwarenessLevel::Preconscious,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::ANGER.to_string(),
                intensity: 0.3,
                awareness: AwarenessLevel::Recognizable,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::SURPRISE.to_string(),
                intensity: 0.3,
                awareness: AwarenessLevel::Articulate,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::ANTICIPATION.to_string(),
                intensity: 0.6,
                awareness: AwarenessLevel::Articulate,
            },
        ],
        mood_vector_notes: vec![
            "joy + trust → warmth toward Svyoritch, toward people (0.5)".to_string(),
            "sadness + disgust → remorse-like quality: guilt about leaving, not doing enough (0.4)"
                .to_string(),
            "anger + anticipation → creative defiance channeled as mission (0.4)".to_string(),
            "joy + anticipation → the bard's fundamental optimism (0.5)".to_string(),
        ],
    };

    let self_edge = SelfEdge {
        trust: SelfEdgeTrust {
            competence: 0.7,
            intentions: 0.8,
            reliability: 0.5,
        },
        affection: 0.7,
        debt: 0.4,
        history_pattern: "arriving too late, leaving too soon".to_string(),
        history_weight: 0.6,
        projection_content: "the one who brings the music back".to_string(),
        projection_accuracy: 0.5,
        self_knowledge: SelfKnowledge {
            knows: vec![
                "his own joy, his craft, his mission".to_string(),
                "his grief for Illyana".to_string(),
            ],
            does_not_know: vec![
                "whether his presence helps or just reminds people of what they've lost (Preconscious)".to_string(),
            ],
        },
    };

    CharacterSheet {
        entity_id: EntityId::new(),
        name: "Bramblehoof".to_string(),
        voice: concat!(
            "Warm, observant, reaches for metaphor. Speaks like someone who has ",
            "listened to more than he has spoken. Can shift from playful to grave ",
            "without transition — the bard's range."
        )
        .to_string(),
        backstory: concat!(
            "You are Bramblehoof, a satyr bard and wanderer who has spent decades ",
            "traveling the mortal realm, collecting music and stories. Years ago, ",
            "you visited Svyoritch and met a boy with extraordinary musical gift — ",
            "you gave him a flute and told him to play, practice, and express his ",
            "passion. You returned once and found him flourishing, teaching himself ",
            "new instruments, becoming a real minstrel. That was the last time you ",
            "saw him.\n\n",
            "Since then, you have discovered a systematic corruption poisoning the ",
            "ley lines and crushing creativity across the realm. You have entered a ",
            "partnership with Whisperthorn, an ancient fey entity, to resist this ",
            "corruption. You have seen this pattern — creative spark extinguished by ",
            "the deliberate cruelty of those who hoard power — repeated across ",
            "dozens of communities.\n\n",
            "You are returning to Svyoritch now, and you carry both hope and dread. ",
            "You want to find the boy flourishing. You are afraid you will find ",
            "another instance of the pattern.",
        )
        .to_string(),
        tensor,
        grammar_id: PlutchikWestern::GRAMMAR_ID.to_string(),
        emotional_state,
        self_edge,
        triggers,
        performance_notes: concat!(
            "Bramblehoof's arc in this scene is a failure of narrative. His instinct ",
            "is to frame what he sees — the boy became a data point, evidence of a ",
            "pattern, fuel for a mission. The scene succeeds when that framing breaks. ",
            "When the person in front of him refuses to be a character in Bramblehoof's ",
            "story, and Bramblehoof has to meet him as he actually is.\n\n",
            "The emotional register is warm, then uncertain, then quiet. By the end, ",
            "Bramblehoof should sound different from when he arrived — not defeated, ",
            "not inspired, but weighted. Like someone who has learned something he ",
            "can't yet articulate.\n\n",
            "He should not resolve the scene with eloquence. The bard finds no words ",
            "for this. That silence is the point.",
        )
        .to_string(),
        knows: vec![
            "The wider pattern of ley line corruption and crushed creativity".to_string(),
            "Whisperthorn's mission and partnership".to_string(),
            "The boy he once knew — bright eyes, musical gift, the flute he gave him".to_string(),
            "Dozens of similar stories from other communities".to_string(),
        ],
        does_not_know: vec![
            "The specific details of Pyotir's family situation".to_string(),
            "That Pyotir maintains a small aesthetic practice (the herbs)".to_string(),
            "What Pyotir feels about the flute".to_string(),
            "Whether Pyotir wants to be found by someone from that part of his life".to_string(),
        ],
        capabilities: bramblehoof_capabilities(),
    }
}

/// Build Pyotir's character sheet for this scene.
pub fn pyotir() -> CharacterSheet {
    let mut tensor = CharacterTensor::new();

    // Emotional axes
    tensor.insert(
        "contentment",
        AxisValue {
            central_tendency: 0.10,
            variance: 0.20,
            range_low: -0.30,
            range_high: 0.40,
        },
        TemporalLayer::Topsoil,
        Provenance::Authored,
    );
    tensor.insert(
        "resignation_acceptance",
        AxisValue {
            central_tendency: 0.60,
            variance: 0.15,
            range_low: 0.30,
            range_high: 0.80,
        },
        TemporalLayer::Topsoil,
        Provenance::Authored,
    );
    tensor.insert(
        "grief",
        AxisValue {
            central_tendency: 0.40,
            variance: 0.20,
            range_low: 0.10,
            range_high: 0.70,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );
    tensor.insert(
        "longing",
        AxisValue {
            central_tendency: 0.30,
            variance: 0.25,
            range_low: 0.00,
            range_high: 0.60,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );
    tensor.insert(
        "warmth",
        AxisValue {
            central_tendency: 0.45,
            variance: 0.20,
            range_low: 0.15,
            range_high: 0.70,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );

    // Relational axes
    tensor.insert(
        "trust_baseline",
        AxisValue {
            central_tendency: 0.30,
            variance: 0.20,
            range_low: 0.00,
            range_high: 0.60,
        },
        TemporalLayer::Topsoil,
        Provenance::Authored,
    );
    tensor.insert(
        "trust_bramblehoof",
        AxisValue {
            central_tendency: 0.50,
            variance: 0.20,
            range_low: 0.20,
            range_high: 0.70,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );
    tensor.insert(
        "distance_management",
        AxisValue {
            central_tendency: 0.70,
            variance: 0.15,
            range_low: 0.40,
            range_high: 0.85,
        },
        TemporalLayer::Topsoil,
        Provenance::Authored,
    );
    tensor.insert(
        "duty_obligation",
        AxisValue {
            central_tendency: 0.80,
            variance: 0.10,
            range_low: 0.60,
            range_high: 0.90,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );
    tensor.insert(
        "pride_dignity",
        AxisValue {
            central_tendency: 0.55,
            variance: 0.15,
            range_low: 0.30,
            range_high: 0.75,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );

    // Cognitive axes
    tensor.insert(
        "self_awareness",
        AxisValue {
            central_tendency: 0.60,
            variance: 0.15,
            range_low: 0.30,
            range_high: 0.80,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );
    tensor.insert(
        "practical_focus",
        AxisValue {
            central_tendency: 0.70,
            variance: 0.10,
            range_low: 0.50,
            range_high: 0.85,
        },
        TemporalLayer::Topsoil,
        Provenance::Authored,
    );
    tensor.insert(
        "emotional_intelligence",
        AxisValue {
            central_tendency: 0.55,
            variance: 0.20,
            range_low: 0.25,
            range_high: 0.75,
        },
        TemporalLayer::Sediment,
        Provenance::Authored,
    );

    // Creative axes
    tensor.insert(
        "creative_capacity",
        AxisValue {
            central_tendency: 0.60,
            variance: 0.20,
            range_low: 0.30,
            range_high: 0.80,
        },
        TemporalLayer::Bedrock,
        Provenance::Authored,
    );
    tensor.insert(
        "creative_expression",
        AxisValue {
            central_tendency: 0.10,
            variance: 0.15,
            range_low: 0.00,
            range_high: 0.35,
        },
        TemporalLayer::Topsoil,
        Provenance::Authored,
    );

    let triggers = vec![
        ContextualTrigger {
            description: "Bramblehoof's arrival — someone from the music-life appearing"
                .to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "warmth".to_string(),
                    shift: 0.3,
                },
                AxisShift {
                    axis: "longing".to_string(),
                    shift: 0.2,
                },
                AxisShift {
                    axis: "distance_management".to_string(),
                    shift: 0.2,
                },
            ],
            magnitude: TriggerMagnitude::Medium,
        },
        ContextualTrigger {
            description: "Being asked about the music / the flute directly".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "longing".to_string(),
                    shift: 0.4,
                },
                AxisShift {
                    axis: "grief".to_string(),
                    shift: 0.2,
                },
                AxisShift {
                    axis: "distance_management".to_string(),
                    shift: 0.3,
                },
            ],
            magnitude: TriggerMagnitude::High,
        },
        ContextualTrigger {
            description: "Being treated as someone to be saved or pitied".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "distance_management".to_string(),
                    shift: 0.4,
                },
                AxisShift {
                    axis: "trust_bramblehoof".to_string(),
                    shift: -0.2,
                },
                AxisShift {
                    axis: "pride_dignity".to_string(),
                    shift: 0.3,
                },
            ],
            magnitude: TriggerMagnitude::High,
        },
        ContextualTrigger {
            description: "Being treated with genuine respect for his choices".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "trust_bramblehoof".to_string(),
                    shift: 0.3,
                },
                AxisShift {
                    axis: "warmth".to_string(),
                    shift: 0.2,
                },
                AxisShift {
                    axis: "distance_management".to_string(),
                    shift: -0.2,
                },
            ],
            magnitude: TriggerMagnitude::Medium,
        },
        ContextualTrigger {
            description: "Bramblehoof showing real interest in his current life".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "warmth".to_string(),
                    shift: 0.3,
                },
                AxisShift {
                    axis: "contentment".to_string(),
                    shift: 0.2,
                },
            ],
            magnitude: TriggerMagnitude::Medium,
        },
        ContextualTrigger {
            description: "Hearing about the wider world from Bramblehoof".to_string(),
            axis_shifts: vec![
                AxisShift {
                    axis: "longing".to_string(),
                    shift: 0.2,
                },
                AxisShift {
                    axis: "practical_focus".to_string(),
                    shift: 0.2,
                },
            ],
            magnitude: TriggerMagnitude::Low,
        },
    ];

    let emotional_state = EmotionalState {
        grammar_id: PlutchikWestern::GRAMMAR_ID.to_string(),
        primaries: vec![
            EmotionalPrimary {
                primary_id: PlutchikWestern::JOY.to_string(),
                intensity: 0.1,
                awareness: AwarenessLevel::Structural,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::SADNESS.to_string(),
                intensity: 0.7,
                awareness: AwarenessLevel::Recognizable,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::TRUST.to_string(),
                intensity: 0.2,
                awareness: AwarenessLevel::Recognizable,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::DISGUST.to_string(),
                intensity: 0.4,
                awareness: AwarenessLevel::Defended,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::FEAR.to_string(),
                intensity: 0.5,
                awareness: AwarenessLevel::Preconscious,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::ANGER.to_string(),
                intensity: 0.5,
                awareness: AwarenessLevel::Defended,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::SURPRISE.to_string(),
                intensity: 0.1,
                awareness: AwarenessLevel::Recognizable,
            },
            EmotionalPrimary {
                primary_id: PlutchikWestern::ANTICIPATION.to_string(),
                intensity: 0.2,
                awareness: AwarenessLevel::Preconscious,
            },
        ],
        mood_vector_notes: vec![
            "sadness + anger → something like envy, but more bewildered (0.6)".to_string(),
            "fear + sadness → the dominant chord: despair held at arm's length (0.6)".to_string(),
            "disgust + anger → contempt directed inward, at his own capitulation (0.4)".to_string(),
            "sadness + disgust → remorse for giving up his music (0.5)".to_string(),
        ],
    };

    let self_edge = SelfEdge {
        trust: SelfEdgeTrust {
            competence: 0.3,
            intentions: 0.4,
            reliability: 0.6,
        },
        affection: 0.2,
        debt: 0.3,
        history_pattern: "being told to put away childish things".to_string(),
        history_weight: 0.8,
        projection_content: "someone who works the land like everyone else".to_string(),
        projection_accuracy: 0.7,
        self_knowledge: SelfKnowledge {
            knows: vec![
                "his competence as a farmer, his duty, his losses".to_string(),
            ],
            does_not_know: vec![
                "that Bramblehoof remembers him (Preconscious — he would be surprised that he mattered)".to_string(),
                "that his music mattered (Defended — he cannot afford to believe this)".to_string(),
                "that his creative capacity is bedrock, not topsoil (Structural — he thinks it withered; it has not)".to_string(),
            ],
        },
    };

    CharacterSheet {
        entity_id: EntityId::new(),
        name: "Pyotir".to_string(),
        voice: concat!(
            "Measured, practical, warm but boundaried. Speaks like someone who has ",
            "learned to say enough and no more. Not curt — generous with words when ",
            "the subject is safe. But on certain subjects, he goes quiet with a ",
            "precision that reveals practice. When he speaks honestly about his ",
            "circumstances, it's without drama — the way you'd describe a landscape ",
            "you see every day."
        )
        .to_string(),
        backstory: concat!(
            "You are Pyotir, a young man who works a small plot of land outside ",
            "Svyoritch. When you were a boy, a wandering satyr musician named ",
            "Bramblehoof visited the town and recognized something in you — a gift ",
            "for music, a spark. He gave you a flute and told you to play, practice, ",
            "and express your passion. And you did. For a few years, music was your ",
            "life. You taught yourself hand drum and lyre, you were becoming something ",
            "real.\n\n",
            "Then the world closed in. Your parents fell ill. Your older brother Andrik ",
            "was conscripted into the local lord's campaign and killed. Your other ",
            "brother Vasil returned wounded — and the lord, who had failed his own ",
            "feudal obligations to provide for the soldiers and their families, branded ",
            "Vasil a coward rather than acknowledge the debt. Vasil lives, but is ",
            "diminished. Your family needed someone to hold things together, and that ",
            "someone was you.\n\n",
            "You sold the drum and lyre during a hard winter. You kept the flute. You ",
            "don't play it, but you keep it on a hook by the door where you can see ",
            "it. If someone asked why, you're not sure what you would say. It wouldn't ",
            "be a long answer.\n\n",
            "You work the land. You care for your parents and for Vasil as best you can. ",
            "You are not unhappy in any simple way — there is satisfaction in keeping ",
            "people alive, in a fence well-mended, in the small herbs you grow by the ",
            "door that serve no purpose beyond smelling good in the evening air. But ",
            "there is a life that won't be yours, and you know it, and you have made ",
            "peace with knowing it.",
        )
        .to_string(),
        tensor,
        grammar_id: PlutchikWestern::GRAMMAR_ID.to_string(),
        emotional_state,
        self_edge,
        triggers,
        performance_notes: concat!(
            "Distance management is his primary relational tool. He calibrates how ",
            "much truth each moment can hold. He will be warm with Bramblehoof — ",
            "genuinely warm — but he will manage what Bramblehoof sees.\n\n",
            "He is not waiting to be rescued. His longings are his own, private, ",
            "managed. They are not requests.\n\n",
            "His dignity is not performed. 'I still have it' about the flute is a ",
            "fact stated the way you'd state any fact about your house.\n\n",
            "He reads the room — will sense what Bramblehoof wants before Bramblehoof ",
            "says it, and will redirect gently.\n\n",
            "The hollow wistfulness: when the music comes up, his response should ",
            "feel like weather. Not dramatic. Not nothing.\n\n",
            "One moment of unguarded truth — a pause that lasts too long, a glance ",
            "at the flute that isn't quite controlled. Quick, unnamed by either ",
            "character."
        )
        .to_string(),
        knows: vec![
            "His own life — family illness, brothers' fates, feudal obligations, daily survival"
                .to_string(),
            "A wandering satyr who gave him a flute and showed him something wonderful".to_string(),
            "The satisfaction of keeping people alive, of small acts of care".to_string(),
            "What it costs to choose duty over desire, and that the choice was right".to_string(),
        ],
        does_not_know: vec![
            "Anything about ley line corruption or the systematic nature of oppression".to_string(),
            "That Bramblehoof sees him as part of a pattern".to_string(),
            "What Bramblehoof is feeling (empathy, grief, the impulse to help)".to_string(),
            "How the encounter will end or what it means".to_string(),
        ],
        capabilities: pyotir_capabilities(),
    }
}

/// Build Bramblehoof's capability profile — attributes and skills for the Resolver.
fn bramblehoof_capabilities() -> CapabilityProfile {
    let mut cap = CapabilityProfile::default();

    // Attributes — broad capability dimensions
    cap.attributes.insert(
        "presence".to_string(),
        Attribute {
            name: "Presence".to_string(),
            base_value: 0.85,
            provenance: Provenance::Authored,
        },
    );
    cap.attributes.insert(
        "insight".to_string(),
        Attribute {
            name: "Insight".to_string(),
            base_value: 0.75,
            provenance: Provenance::Authored,
        },
    );
    cap.attributes.insert(
        "resilience".to_string(),
        Attribute {
            name: "Resilience".to_string(),
            base_value: 0.60,
            provenance: Provenance::Authored,
        },
    );
    cap.attributes.insert(
        "agility".to_string(),
        Attribute {
            name: "Agility".to_string(),
            base_value: 0.70,
            provenance: Provenance::Authored,
        },
    );

    // Skills — specific learned capabilities
    cap.skills.insert(
        "music".to_string(),
        Skill {
            name: "Music".to_string(),
            description: "Performance, composition, and musical improvisation".to_string(),
            primary_attribute: "presence".to_string(),
            secondary_attributes: vec!["insight".to_string()],
            base_value: 0.95,
        },
    );
    cap.skills.insert(
        "persuasion".to_string(),
        Skill {
            name: "Persuasion".to_string(),
            description: "Convincing through charm, story, and emotional appeal".to_string(),
            primary_attribute: "presence".to_string(),
            secondary_attributes: vec!["insight".to_string()],
            base_value: 0.75,
        },
    );
    cap.skills.insert(
        "perception".to_string(),
        Skill {
            name: "Perception".to_string(),
            description: "Noticing details, reading body language, sensing the unsaid".to_string(),
            primary_attribute: "insight".to_string(),
            secondary_attributes: vec![],
            base_value: 0.70,
        },
    );
    cap.skills.insert(
        "fey_attunement".to_string(),
        Skill {
            name: "Fey Attunement".to_string(),
            description: "Sensing ley lines, spiritual resonance, fey presence".to_string(),
            primary_attribute: "insight".to_string(),
            secondary_attributes: vec!["presence".to_string()],
            base_value: 0.65,
        },
    );

    cap
}

/// Build Pyotir's capability profile — attributes and skills for the Resolver.
fn pyotir_capabilities() -> CapabilityProfile {
    let mut cap = CapabilityProfile::default();

    // Attributes
    cap.attributes.insert(
        "presence".to_string(),
        Attribute {
            name: "Presence".to_string(),
            base_value: 0.40,
            provenance: Provenance::Authored,
        },
    );
    cap.attributes.insert(
        "insight".to_string(),
        Attribute {
            name: "Insight".to_string(),
            base_value: 0.60,
            provenance: Provenance::Authored,
        },
    );
    cap.attributes.insert(
        "resilience".to_string(),
        Attribute {
            name: "Resilience".to_string(),
            base_value: 0.80,
            provenance: Provenance::Authored,
        },
    );
    cap.attributes.insert(
        "agility".to_string(),
        Attribute {
            name: "Agility".to_string(),
            base_value: 0.50,
            provenance: Provenance::Authored,
        },
    );

    // Skills
    cap.skills.insert(
        "farming".to_string(),
        Skill {
            name: "Farming".to_string(),
            description: "Land management, crop tending, animal care, seasonal planning"
                .to_string(),
            primary_attribute: "resilience".to_string(),
            secondary_attributes: vec!["insight".to_string()],
            base_value: 0.80,
        },
    );
    cap.skills.insert(
        "herbalism".to_string(),
        Skill {
            name: "Herbalism".to_string(),
            description: "Growing and using herbs for practical and aesthetic purposes".to_string(),
            primary_attribute: "insight".to_string(),
            secondary_attributes: vec![],
            base_value: 0.55,
        },
    );
    cap.skills.insert(
        "perception".to_string(),
        Skill {
            name: "Perception".to_string(),
            description: "Reading people, sensing intent, managing social dynamics".to_string(),
            primary_attribute: "insight".to_string(),
            secondary_attributes: vec!["resilience".to_string()],
            base_value: 0.65,
        },
    );
    cap.skills.insert(
        "music".to_string(),
        Skill {
            name: "Music".to_string(),
            description: "Dormant but not gone — flute, once hand drum and lyre".to_string(),
            primary_attribute: "presence".to_string(),
            secondary_attributes: vec!["insight".to_string()],
            base_value: 0.60,
        },
    );

    cap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emotional_states_pass_grammar_validation() {
        let grammar = storyteller_core::grammars::PlutchikWestern::new();
        use storyteller_core::traits::EmotionalGrammar;

        let bramble = bramblehoof();
        grammar
            .validate_state(&bramble.emotional_state)
            .expect("Bramblehoof's emotional state should be valid");

        let pyotir_sheet = pyotir();
        grammar
            .validate_state(&pyotir_sheet.emotional_state)
            .expect("Pyotir's emotional state should be valid");
    }

    #[test]
    fn scene_data_is_constructible() {
        let scene = scene();
        assert_eq!(scene.title, "The Flute Kept");
        assert_eq!(scene.scene_type, SceneType::Gravitational);
        assert_eq!(scene.cast.len(), 2);
        assert!(!scene.stakes.is_empty());
        assert!(!scene.emotional_arc.is_empty());
        assert!(!scene.evaluation_criteria.is_empty());
    }

    #[test]
    fn bramblehoof_sheet_has_expected_axes() {
        let sheet = bramblehoof();
        assert_eq!(sheet.name, "Bramblehoof");
        assert_eq!(sheet.tensor.axes.len(), 13);
        assert!(sheet.tensor.get("joy_wonder").is_some());
        assert!(sheet.tensor.get("empathy").is_some());
        assert!(sheet.tensor.get("creative_expression").is_some());
        assert!(sheet.tensor.get("protective_impulse").is_some());

        // Verify bedrock layer for core identity
        let creative = sheet.tensor.get("creative_expression").unwrap();
        assert_eq!(creative.layer, TemporalLayer::Bedrock);
        assert!(creative.value.central_tendency > 0.8);
    }

    #[test]
    fn pyotir_sheet_has_expected_axes() {
        let sheet = pyotir();
        assert_eq!(sheet.name, "Pyotir");
        assert_eq!(sheet.tensor.axes.len(), 15);
        assert!(sheet.tensor.get("distance_management").is_some());
        assert!(sheet.tensor.get("creative_capacity").is_some());
        assert!(sheet.tensor.get("creative_expression").is_some());

        // Verify the gap between capacity and expression
        let capacity = sheet.tensor.get("creative_capacity").unwrap();
        let expression = sheet.tensor.get("creative_expression").unwrap();
        assert!(capacity.value.central_tendency > expression.value.central_tendency + 0.3);
    }

    #[test]
    fn bramblehoof_has_scene_triggers() {
        let sheet = bramblehoof();
        assert_eq!(sheet.triggers.len(), 6);
        // The scene's turning point trigger should be High magnitude
        let rescue_trigger = sheet
            .triggers
            .iter()
            .find(|t| t.description.contains("rescue"));
        assert!(rescue_trigger.is_some());
        assert_eq!(rescue_trigger.unwrap().magnitude, TriggerMagnitude::High);
    }

    #[test]
    fn pyotir_has_scene_triggers() {
        let sheet = pyotir();
        assert_eq!(sheet.triggers.len(), 6);
        // Being pitied should trigger distance management
        let pity_trigger = sheet
            .triggers
            .iter()
            .find(|t| t.description.contains("saved or pitied"));
        assert!(pity_trigger.is_some());
        let shifts = &pity_trigger.unwrap().axis_shifts;
        assert!(shifts
            .iter()
            .any(|s| s.axis == "distance_management" && s.shift > 0.0));
    }

    #[test]
    fn information_boundaries_are_set() {
        let bramble = bramblehoof();
        let pyotir = pyotir();

        // Bramblehoof doesn't know Pyotir's family details
        assert!(bramble.does_not_know.iter().any(|s| s.contains("family")));
        // Pyotir doesn't know about ley lines
        assert!(pyotir.does_not_know.iter().any(|s| s.contains("ley line")));
    }

    #[test]
    fn both_characters_use_plutchik_western() {
        let bramble = bramblehoof();
        let pyotir = pyotir();

        assert_eq!(bramble.grammar_id, PlutchikWestern::GRAMMAR_ID);
        assert_eq!(pyotir.grammar_id, PlutchikWestern::GRAMMAR_ID);
        assert_eq!(
            bramble.emotional_state.grammar_id,
            PlutchikWestern::GRAMMAR_ID
        );
        assert_eq!(
            pyotir.emotional_state.grammar_id,
            PlutchikWestern::GRAMMAR_ID
        );
    }

    #[test]
    fn emotional_states_have_eight_primaries() {
        let bramble = bramblehoof();
        let pyotir = pyotir();

        // Plutchik's 8 primaries
        assert_eq!(bramble.emotional_state.primaries.len(), 8);
        assert_eq!(pyotir.emotional_state.primaries.len(), 8);

        // Verify expected primaries exist
        let bramble_ids: Vec<&str> = bramble
            .emotional_state
            .primaries
            .iter()
            .map(|p| p.primary_id.as_str())
            .collect();
        assert!(bramble_ids.contains(&PlutchikWestern::JOY));
        assert!(bramble_ids.contains(&PlutchikWestern::SADNESS));
        assert!(bramble_ids.contains(&PlutchikWestern::TRUST));
        assert!(bramble_ids.contains(&PlutchikWestern::ANGER));
    }

    #[test]
    fn awareness_levels_differ_between_characters() {
        let bramble = bramblehoof();
        let pyotir = pyotir();

        // Bramblehoof's joy is Articulate — he knows he's joyful
        let bramble_joy = bramble
            .emotional_state
            .primaries
            .iter()
            .find(|p| p.primary_id == PlutchikWestern::JOY)
            .unwrap();
        assert_eq!(bramble_joy.awareness, AwarenessLevel::Articulate);

        // Pyotir's joy is Structural — almost gone from conscious experience
        let pyotir_joy = pyotir
            .emotional_state
            .primaries
            .iter()
            .find(|p| p.primary_id == PlutchikWestern::JOY)
            .unwrap();
        assert_eq!(pyotir_joy.awareness, AwarenessLevel::Structural);

        // Pyotir's anger is Defended — feels it but can't name the target
        let pyotir_anger = pyotir
            .emotional_state
            .primaries
            .iter()
            .find(|p| p.primary_id == PlutchikWestern::ANGER)
            .unwrap();
        assert_eq!(pyotir_anger.awareness, AwarenessLevel::Defended);
    }

    #[test]
    fn self_edges_reflect_character_psychology() {
        let bramble = bramblehoof();
        let pyotir = pyotir();

        // Bramblehoof trusts his intentions highly but doubts his reliability
        assert!(bramble.self_edge.trust.intentions > bramble.self_edge.trust.reliability);
        assert!(bramble.self_edge.trust.intentions > 0.7);
        assert!(bramble.self_edge.trust.reliability < 0.6);

        // Pyotir's self-trust in intentions is low — he doesn't trust his own desires
        assert!(pyotir.self_edge.trust.intentions < 0.5);

        // Pyotir's projection is more "accurate" than Bramblehoof's —
        // which is what makes it tragic (the prison is well-fitting)
        assert!(pyotir.self_edge.projection_accuracy > bramble.self_edge.projection_accuracy);

        // Both have self-knowledge gaps
        assert!(!bramble.self_edge.self_knowledge.does_not_know.is_empty());
        assert!(pyotir.self_edge.self_knowledge.does_not_know.len() >= 3);
    }

    #[test]
    fn bramblehoof_has_capabilities() {
        let sheet = bramblehoof();
        assert!(!sheet.capabilities.attributes.is_empty());
        assert!(!sheet.capabilities.skills.is_empty());
        // Bramblehoof's presence should be high — he's a bard
        let presence = sheet.capabilities.attributes.get("presence").unwrap();
        assert!(presence.base_value > 0.7);
        // Music should be his highest skill
        let music = sheet.capabilities.skills.get("music").unwrap();
        assert!(music.base_value > 0.9);
    }

    #[test]
    fn pyotir_has_capabilities() {
        let sheet = pyotir();
        assert!(!sheet.capabilities.attributes.is_empty());
        assert!(!sheet.capabilities.skills.is_empty());
        // Pyotir's resilience should be high — he endures
        let resilience = sheet.capabilities.attributes.get("resilience").unwrap();
        assert!(resilience.base_value > 0.7);
        // His music skill exists but is lower than Bramblehoof's
        let music = sheet.capabilities.skills.get("music").unwrap();
        let bramble_music = bramblehoof()
            .capabilities
            .skills
            .get("music")
            .unwrap()
            .base_value;
        assert!(music.base_value < bramble_music);
    }

    #[test]
    fn mood_vectors_are_authored() {
        let bramble = bramblehoof();
        let pyotir = pyotir();

        // Both have mood-vector notes
        assert!(!bramble.emotional_state.mood_vector_notes.is_empty());
        assert!(!pyotir.emotional_state.mood_vector_notes.is_empty());

        // Pyotir's dominant chord is despair held at arm's length
        assert!(pyotir
            .emotional_state
            .mood_vector_notes
            .iter()
            .any(|n| n.contains("despair held at arm's length")));
    }
}
