"""Tests for dataset.py — JSONL loading and cell-stratified splitting."""

from pathlib import Path

import torch

from training.dataset import (
    CharacterDataset,
    cell_key,
    create_dataloaders,
    load_jsonl,
    stratified_split,
)
from training.feature_schema import TOTAL_INPUT_FEATURES, TOTAL_OUTPUT_FEATURES


def test_cell_key_deterministic():
    cell = {
        "archetype_a": "guardian",
        "archetype_b": "trickster",
        "dynamic": "trust_building",
        "profile": "high_tension",
        "a_is_role_a": True,
    }
    k1 = cell_key(cell)
    k2 = cell_key(cell)
    assert k1 == k2
    assert "guardian" in k1
    assert "trickster" in k1


def test_cell_key_differs_on_role_swap():
    cell_a = {
        "archetype_a": "guardian",
        "archetype_b": "trickster",
        "dynamic": "trust_building",
        "profile": "high_tension",
        "a_is_role_a": True,
    }
    cell_b = {**cell_a, "a_is_role_a": False}
    assert cell_key(cell_a) != cell_key(cell_b)


def test_load_jsonl(synthetic_jsonl: Path):
    features, labels, cells = load_jsonl(synthetic_jsonl)
    assert features.shape == (100, TOTAL_INPUT_FEATURES)
    assert labels.shape == (100, TOTAL_OUTPUT_FEATURES)
    assert len(cells) == 100
    assert features.dtype == torch.float32


def test_stratified_split_no_cell_leak(synthetic_jsonl: Path):
    features, labels, cells = load_jsonl(synthetic_jsonl)
    train_f, train_l, val_f, val_l = stratified_split(features, labels, cells)

    # All examples accounted for
    assert train_f.shape[0] + val_f.shape[0] == 100

    # Reconstruct cell keys for each partition
    train_indices = set()
    val_indices = set()
    for i in range(100):
        in_train = any(torch.equal(features[i], train_f[j]) for j in range(train_f.shape[0]))
        if in_train:
            train_indices.add(i)
        else:
            val_indices.add(i)

    train_cells = {cells[i] for i in train_indices}
    val_cells = {cells[i] for i in val_indices}

    # No cell should appear in both partitions
    assert train_cells.isdisjoint(val_cells), "Cell leak: some cells appear in both splits"


def test_stratified_split_respects_fraction(synthetic_jsonl: Path):
    features, labels, cells = load_jsonl(synthetic_jsonl)
    _, _, val_f, _ = stratified_split(features, labels, cells, val_fraction=0.2)

    # With 100 examples from 10 cells of 10 each, 20% means ~2 cells = 20 examples
    # Allow some tolerance since we split by cell
    assert 10 <= val_f.shape[0] <= 40


def test_character_dataset():
    features = torch.randn(50, TOTAL_INPUT_FEATURES)
    labels = torch.randn(50, TOTAL_OUTPUT_FEATURES)
    ds = CharacterDataset(features, labels)
    assert len(ds) == 50
    feat, lab = ds[0]
    assert feat.shape == (TOTAL_INPUT_FEATURES,)
    assert lab.shape == (TOTAL_OUTPUT_FEATURES,)


def test_create_dataloaders():
    train_ds = CharacterDataset(
        torch.randn(80, TOTAL_INPUT_FEATURES),
        torch.randn(80, TOTAL_OUTPUT_FEATURES),
    )
    val_ds = CharacterDataset(
        torch.randn(20, TOTAL_INPUT_FEATURES),
        torch.randn(20, TOTAL_OUTPUT_FEATURES),
    )
    train_loader, val_loader = create_dataloaders(train_ds, val_ds, batch_size=16)

    batch_f, batch_l = next(iter(train_loader))
    assert batch_f.shape == (16, TOTAL_INPUT_FEATURES)
    assert batch_l.shape == (16, TOTAL_OUTPUT_FEATURES)
