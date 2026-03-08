"""Training loop with early stopping and checkpointing."""

from dataclasses import dataclass, field
from pathlib import Path

import torch
from torch.optim import AdamW
from torch.optim.lr_scheduler import ReduceLROnPlateau

from training.dataset import (
    CharacterDataset,
    create_dataloaders,
    load_jsonl,
    stratified_split,
)
from training.losses import MultiHeadLoss
from training.metrics import MetricsAccumulator
from training.model import CharacterPredictor


def detect_device() -> torch.device:
    """Detect best available device: MPS → CUDA → CPU."""
    if torch.backends.mps.is_available():
        return torch.device("mps")
    if torch.cuda.is_available():
        return torch.device("cuda")
    print("WARNING: No GPU detected, training on CPU (this will be slow)")
    return torch.device("cpu")


@dataclass
class TrainConfig:
    epochs: int = 100
    batch_size: int = 256
    lr: float = 1e-3
    weight_decay: float = 1e-4
    patience: int = 10
    min_delta: float = 1e-4
    val_fraction: float = 0.2
    seed: int = 42
    dropout: float = 0.3
    action_weight: float = 0.35
    speech_weight: float = 0.20
    thought_weight: float = 0.20
    emotion_weight: float = 0.25
    checkpoint_dir: Path = field(default_factory=lambda: Path("checkpoints"))


def train(config: TrainConfig, data_path: str | Path) -> Path:
    """Train the character predictor model. Returns path to best checkpoint."""
    torch.manual_seed(config.seed)
    device = detect_device()
    print(f"Training on device: {device}")

    # Load and split data
    print(f"Loading data from {data_path}...")
    features, labels, cells = load_jsonl(data_path)
    train_f, train_l, val_f, val_l = stratified_split(
        features, labels, cells, val_fraction=config.val_fraction, seed=config.seed
    )
    print(f"  Train: {train_f.shape[0]} examples, Val: {val_f.shape[0]} examples")

    train_ds = CharacterDataset(train_f, train_l)
    val_ds = CharacterDataset(val_f, val_l)
    train_loader, val_loader = create_dataloaders(train_ds, val_ds, batch_size=config.batch_size)

    # Build model
    model = CharacterPredictor(dropout=config.dropout).to(device)
    total_params = sum(p.numel() for p in model.parameters())
    print(f"  Model parameters: {total_params:,}")

    loss_fn = MultiHeadLoss(
        action_weight=config.action_weight,
        speech_weight=config.speech_weight,
        thought_weight=config.thought_weight,
        emotion_weight=config.emotion_weight,
    ).to(device)

    optimizer = AdamW(model.parameters(), lr=config.lr, weight_decay=config.weight_decay)
    scheduler = ReduceLROnPlateau(optimizer, mode="min", factor=0.5, patience=5)

    # Checkpointing setup
    config.checkpoint_dir.mkdir(parents=True, exist_ok=True)
    best_path = config.checkpoint_dir / "best_model.pth"
    best_val_loss = float("inf")
    epochs_without_improvement = 0

    metrics_acc = MetricsAccumulator()

    for epoch in range(1, config.epochs + 1):
        # --- Training ---
        model.train()
        train_loss_sum = 0.0
        train_batches = 0

        for batch_features, batch_labels in train_loader:
            batch_features = batch_features.to(device)
            batch_labels = batch_labels.to(device)

            optimizer.zero_grad()
            predictions = model(batch_features)
            losses = loss_fn(predictions, batch_labels)
            losses["total"].backward()
            optimizer.step()

            train_loss_sum += losses["total"].item()
            train_batches += 1

        train_loss = train_loss_sum / train_batches

        # --- Validation ---
        model.eval()
        val_loss_sum = 0.0
        val_batches = 0
        metrics_acc.reset()

        with torch.no_grad():
            for batch_features, batch_labels in val_loader:
                batch_features = batch_features.to(device)
                batch_labels = batch_labels.to(device)

                predictions = model(batch_features)
                losses = loss_fn(predictions, batch_labels)

                val_loss_sum += losses["total"].item()
                val_batches += 1
                metrics_acc.update(predictions, batch_labels)

        val_loss = val_loss_sum / val_batches
        metrics = metrics_acc.compute()
        scheduler.step(val_loss)

        # Logging
        lr = optimizer.param_groups[0]["lr"]
        print(
            f"Epoch {epoch:3d}/{config.epochs} | "
            f"train_loss={train_loss:.4f} | val_loss={val_loss:.4f} | lr={lr:.2e}"
        )
        print(
            f"  action_type_acc={metrics.get('action_type_acc', 0):.3f}  "
            f"speech_occurs_acc={metrics.get('speech_occurs_acc', 0):.3f}  "
            f"awareness_acc={metrics.get('thought_awareness_acc', 0):.3f}  "
            f"emotion_delta_mse={metrics.get('emotion_delta_mse', 0):.4f}"
        )

        # Early stopping
        if val_loss < best_val_loss - config.min_delta:
            best_val_loss = val_loss
            epochs_without_improvement = 0
            torch.save(
                {
                    "model_state_dict": model.state_dict(),
                    "optimizer_state_dict": optimizer.state_dict(),
                    "epoch": epoch,
                    "val_loss": val_loss,
                    "config": {
                        "dropout": config.dropout,
                        "lr": config.lr,
                        "batch_size": config.batch_size,
                        "action_weight": config.action_weight,
                        "speech_weight": config.speech_weight,
                        "thought_weight": config.thought_weight,
                        "emotion_weight": config.emotion_weight,
                    },
                },
                best_path,
            )
            print(f"  -> Saved best model (val_loss={val_loss:.4f})")
        else:
            epochs_without_improvement += 1
            if epochs_without_improvement >= config.patience:
                print(f"  Early stopping after {config.patience} epochs without improvement")
                break

    print(f"\nBest validation loss: {best_val_loss:.4f}")
    print(f"Best checkpoint: {best_path}")
    return best_path
