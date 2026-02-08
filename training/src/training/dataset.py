"""JSONL loading, cell-stratified splitting, and DataLoaders."""

import json
from collections import defaultdict
from pathlib import Path

import torch
from torch.utils.data import DataLoader, Dataset

from training.feature_schema import TOTAL_INPUT_FEATURES, TOTAL_OUTPUT_FEATURES


def cell_key(cell: dict) -> str:
    """Deterministic key from a cell dict for stratified splitting."""
    return (
        f"{cell['archetype_a']}|{cell['archetype_b']}|"
        f"{cell['dynamic']}|{cell['profile']}|{cell['a_is_role_a']}"
    )


def load_jsonl(
    path: str | Path,
) -> tuple[torch.Tensor, torch.Tensor, list[str]]:
    """Load JSONL file, validate dimensions, return (features, labels, cell_keys).

    Returns float32 tensors and a list of cell keys (one per example).
    """
    features_list: list[list[float]] = []
    labels_list: list[list[float]] = []
    cells: list[str] = []

    path = Path(path)
    with open(path) as f:
        for line_num, line in enumerate(f, 1):
            record = json.loads(line)
            feats = record["features"]
            labs = record["labels"]

            if len(feats) != TOTAL_INPUT_FEATURES:
                raise ValueError(
                    f"Line {line_num}: feature vector has {len(feats)} elements, "
                    f"expected {TOTAL_INPUT_FEATURES}"
                )
            if len(labs) != TOTAL_OUTPUT_FEATURES:
                raise ValueError(
                    f"Line {line_num}: label vector has {len(labs)} elements, "
                    f"expected {TOTAL_OUTPUT_FEATURES}"
                )

            features_list.append(feats)
            labels_list.append(labs)
            cells.append(cell_key(record["cell"]))

    features = torch.tensor(features_list, dtype=torch.float32)
    labels = torch.tensor(labels_list, dtype=torch.float32)
    return features, labels, cells


def stratified_split(
    features: torch.Tensor,
    labels: torch.Tensor,
    cells: list[str],
    val_fraction: float = 0.2,
    seed: int = 42,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
    """Split data by cell key so all variations of a cell go to the same partition.

    Returns (train_features, train_labels, val_features, val_labels).
    """
    # Group example indices by cell key
    cell_indices: dict[str, list[int]] = defaultdict(list)
    for i, key in enumerate(cells):
        cell_indices[key].append(i)

    # Shuffle cell keys deterministically
    unique_cells = sorted(cell_indices.keys())
    rng = torch.Generator().manual_seed(seed)
    perm = torch.randperm(len(unique_cells), generator=rng).tolist()
    shuffled_cells = [unique_cells[i] for i in perm]

    # Assign cells to val until we reach val_fraction
    n_total = len(cells)
    n_val_target = int(n_total * val_fraction)

    val_indices: list[int] = []
    train_indices: list[int] = []
    val_count = 0

    for cell in shuffled_cells:
        indices = cell_indices[cell]
        if val_count < n_val_target:
            val_indices.extend(indices)
            val_count += len(indices)
        else:
            train_indices.extend(indices)

    # Sort for deterministic ordering
    train_indices.sort()
    val_indices.sort()

    return (
        features[train_indices],
        labels[train_indices],
        features[val_indices],
        labels[val_indices],
    )


class CharacterDataset(Dataset):
    """Wraps feature and label tensors as a PyTorch Dataset."""

    def __init__(self, features: torch.Tensor, labels: torch.Tensor) -> None:
        assert features.shape[0] == labels.shape[0]
        self.features = features
        self.labels = labels

    def __len__(self) -> int:
        return self.features.shape[0]

    def __getitem__(self, idx: int) -> tuple[torch.Tensor, torch.Tensor]:
        return self.features[idx], self.labels[idx]


def create_dataloaders(
    train_ds: CharacterDataset,
    val_ds: CharacterDataset,
    batch_size: int = 256,
) -> tuple[DataLoader, DataLoader]:
    """Create shuffled train and sequential val DataLoaders."""
    train_loader = DataLoader(train_ds, batch_size=batch_size, shuffle=True, drop_last=False)
    val_loader = DataLoader(val_ds, batch_size=batch_size, shuffle=False, drop_last=False)
    return train_loader, val_loader
