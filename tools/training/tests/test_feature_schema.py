"""Tests for feature_schema.py — verify constants match Rust schema."""

from training import feature_schema as fs


def test_total_input_features_is_453():
    assert fs.TOTAL_INPUT_FEATURES == 453


def test_total_output_features_is_42():
    assert fs.TOTAL_OUTPUT_FEATURES == 42


def test_input_regions_sum_to_total():
    regions = [
        fs.MAX_TENSOR_AXES * fs.FEATURES_PER_AXIS,  # 208
        fs.NUM_PRIMARIES * fs.FEATURES_PER_PRIMARY,  # 48
        fs.SELF_EDGE_FEATURES,  # 7
        fs.MAX_EDGES * fs.FEATURES_PER_EDGE,  # 120
        fs.SCENE_FEATURES,  # 6
        fs.EVENT_FEATURES,  # 16
        fs.HISTORY_DEPTH * fs.FEATURES_PER_HISTORY,  # 48
    ]
    assert sum(regions) == fs.TOTAL_INPUT_FEATURES


def test_output_head_sizes_sum_to_total():
    assert (
        fs.ACTION_HEAD_SIZE + fs.SPEECH_HEAD_SIZE + fs.THOUGHT_HEAD_SIZE + fs.EMOTION_HEAD_SIZE
    ) == fs.TOTAL_OUTPUT_FEATURES


def test_label_slices_cover_full_vector():
    """All 42 positions should be reachable via the defined slices and indices."""
    covered = set()
    covered.update(range(*fs.ACTION_TYPE_SLICE.indices(42)))
    covered.add(fs.ACTION_CONFIDENCE_IDX)
    covered.add(fs.ACTION_TARGET_IDX)
    covered.add(fs.ACTION_VALENCE_IDX)
    covered.update(range(*fs.ACTION_CONTEXT_SLICE.indices(42)))
    covered.add(fs.SPEECH_OCCURS_IDX)
    covered.update(range(*fs.SPEECH_REGISTER_SLICE.indices(42)))
    covered.add(fs.SPEECH_CONFIDENCE_IDX)
    covered.update(range(*fs.AWARENESS_LEVEL_SLICE.indices(42)))
    covered.add(fs.DOMINANT_EMOTION_IDX)
    covered.update(range(*fs.INTENSITY_DELTA_SLICE.indices(42)))
    covered.update(range(*fs.AWARENESS_SHIFT_SLICE.indices(42)))
    assert covered == set(range(42))


def test_enum_list_lengths():
    assert len(fs.ACTION_TYPES) == fs.NUM_ACTION_TYPES
    assert len(fs.ACTION_CONTEXTS) == fs.NUM_ACTION_CONTEXTS
    assert len(fs.SPEECH_REGISTERS) == fs.NUM_SPEECH_REGISTERS
    assert len(fs.AWARENESS_LEVELS) == fs.NUM_AWARENESS_LEVELS
    assert len(fs.EMOTIONAL_PRIMARIES) == fs.NUM_PRIMARIES


def test_verify_dimensions_passes():
    features = [0.0] * fs.TOTAL_INPUT_FEATURES
    labels = [0.0] * fs.TOTAL_OUTPUT_FEATURES
    fs.verify_dimensions(features, labels)


def test_verify_dimensions_rejects_wrong_features():
    import pytest

    with pytest.raises(AssertionError, match="Feature vector"):
        fs.verify_dimensions([0.0] * 100, [0.0] * 42)


def test_verify_dimensions_rejects_wrong_labels():
    import pytest

    with pytest.raises(AssertionError, match="Label vector"):
        fs.verify_dimensions([0.0] * 453, [0.0] * 10)
