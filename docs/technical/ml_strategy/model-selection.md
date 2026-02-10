# Model Selection

## Two Tasks, Two Architectures

The storyteller ML pipelines use fundamentally different model architectures because the tasks have fundamentally different input characteristics. This is a deliberate choice, not an accident of incremental development.

---

## MLP for Structured Features

The character prediction model is a multi-head MLP: shared trunk (453→384→256→256) with four specialized heads, producing 42 output dimensions. Total ONNX size: 38KB. Inference time: sub-millisecond.

### Why Not a Transformer

The 453-dim input vector is already feature-engineered. Each dimension has a known meaning — axis 47 is the affection component of relational edge 2, not a learned embedding. The features are a flat structure, not a sequence. There are no positional dependencies between dimensions (the empathy axis doesn't "attend to" the scene tension float).

A transformer encoder would add:
- **Parameters**: 44M minimum (DeBERTa-v3-small), vs ~100K for the MLP
- **Latency**: 5-15ms per forward pass, vs <1ms
- **Model size**: ~180MB ONNX, vs 38KB
- **No representational benefit**: Self-attention discovers relationships between sequence positions. Our features are already decomposed into meaningful regions with known interactions.

The right model for structured features is a structured-feature model. The MLP works because the feature engineering already captures the relational structure that a transformer would need to learn.

### When a Transformer Might Help

If the character prediction model ever needs to consume raw text (narrative prose, unstructured backstory descriptions) alongside tensor features, a multimodal architecture with a text encoder would be justified. For now, text processing is the event classifier's job — character prediction receives pre-classified structured input.

---

## Transformer for Text Understanding

The event classifier needs contextual token representations for two tasks: understanding what kind of event a sentence describes (sequence classification) and identifying entity boundaries within sentences (token classification / NER). Both require:

- **Contextual embeddings**: "bank" means something different in "the river bank" vs "the bank account"
- **Subword handling**: Domain vocabulary ("Bramblehoof", "ayahuasca") must be tokenizable without out-of-vocabulary failures
- **Pre-trained language understanding**: The model needs general English comprehension, fine-tuned to narrative domain

Pre-trained transformer encoders provide all three via self-attention and masked language model pre-training.

---

## DeBERTa-v3-small: The Intended Choice

DeBERTa-v3-small (44M parameters) was the first choice for event classification:

- **Disentangled attention**: Separates content and position encodings, computing three attention components (content-to-content, content-to-position, position-to-content). This provides stronger position-aware representations than standard BERT attention.
- **Strong NLU benchmarks**: Competitive with much larger models on SuperGLUE and other NLU tasks.
- **Efficient size**: 44M parameters, ~180MB ONNX — small enough for local inference, large enough for strong NLU.
- **Well-supported**: HuggingFace Transformers integration, established ONNX export path.

DeBERTa was the architecturally superior choice for this task.

---

## Apple MPS NaN Instability

DeBERTa-v3-small training fails on Apple MPS (Metal Performance Shaders) with NaN gradients at approximately epoch 0.59.

### The Problem

DeBERTa's triple attention mechanism (content-to-content, content-to-position, position-to-content) involves multiplying three attention score matrices before softmax normalization. On MPS, the intermediate values exceed float32 precision bounds during the backward pass, producing NaN gradients that poison all subsequent parameter updates.

The failure is deterministic — it occurs at the same training step (±1) across runs. CPU training on the same data with the same hyperparameters completes normally with no numerical issues, confirming this is an MPS-specific precision limitation rather than a data or hyperparameter problem.

### Why Not CPU Training

CPU training on an M4/64GB machine runs at ~0.8 it/s (vs ~6.3 it/s on MPS with DistilBERT). At that rate, 10 epochs over 8,000 examples would take approximately 78 hours. This is not practical for iterative model development.

### Status

DeBERTa-v3-small is shelved until cloud GPU training is available (CUDA does not have this precision issue). The MPS NaN instability is a known limitation of complex attention variants on Apple's Metal backend.

---

## DistilBERT: The Pragmatic Fallback

DistilBERT (66M parameters) was selected as the production encoder:

- **Architecture**: 6-layer transformer encoder, distilled from BERT-base. Standard multi-head self-attention (no disentangled position encoding). 97% of BERT-base performance at 60% faster inference.
- **MPS stability**: Trains without numerical issues at ~6.3 iterations/second on M4/64GB.
- **Proven reliability**: Widely deployed, battle-tested ONNX export, minimal surprises.
- **Sufficient for the task**: Event classification and NER on narrative text do not require the bleeding edge of NLU — DistilBERT's language understanding is more than adequate.

### Deployed Model Sizes

| Model | ONNX Size | Parameters |
|-------|-----------|------------|
| Event classifier | ~268MB | 66M + classification head |
| NER classifier | ~266MB | 66M + token classification head |
| Tokenizer | ~0.7MB | WordPiece vocabulary |

---

## Perfect F1 on Templates

Both deployed models achieve F1 = 1.0 on the template-generated evaluation set after 10 epochs of training.

### Why This Is Expected

The training data is generated from 45 templates with combinatorial vocabulary filling. The patterns are regular — every `ActionOccurrence` example follows recognizable structural patterns ("I pick up {object}", "I walk to {location}"). A transformer with 66M parameters can memorize these patterns easily.

### What It Tells Us

Perfect F1 on templates validates:
- **Pipeline correctness**: Tokenization, BIO label alignment, loss computation, ONNX export, and Rust inference all work end-to-end.
- **Label contract**: The Rust and Python sides agree on label indices — argmax decoding produces the correct labels.
- **Numerical stability**: Training converges without NaN or divergence issues.

It does **not** validate generalization. Real narrative prose — with ambiguity, metaphor, nested clauses, and context-dependent meaning — will challenge the model in ways templates cannot. Phase C.6 (evaluation framework) is designed to test this.

---

## Separate vs Multi-Task Models

### Current: Two Separate Models

The event classifier and NER model are trained independently on different task objectives (sequence classification vs token classification). They share the same DistilBERT architecture and tokenizer but have separate weights.

**Advantages**:
- Independent iteration — can retrain NER without touching event classification
- Independent tuning — different learning rates, batch sizes, epochs
- Simpler debugging — when something goes wrong, the scope is one model
- Independent evaluation — can measure each task's quality separately

### Future: Single Multi-Task Model

A single DistilBERT encoder with two heads (one for sequence classification, one for token classification) would:
- Share encoder gradients between tasks (mutual regularization)
- Reduce total model size (~266MB vs ~534MB)
- Reduce inference latency (one forward pass instead of two)
- Potentially improve both tasks if event kind understanding helps entity extraction and vice versa

The consolidation is planned after Phase C.6 evaluation establishes baselines. Moving to multi-task training before knowing single-task quality makes it harder to diagnose regressions.

---

## ONNX as Abstraction Layer

ONNX provides the boundary between Python training and Rust inference. The export step validates numerical equivalence:

### Export Validation

The Python export script runs both PyTorch and onnxruntime on the same input and compares outputs element-wise:

```
Max absolute difference tolerance: atol=1e-4
Deployed results:
  Event classifier: max diff 4.53e-06
  NER classifier: max diff 4.39e-05
```

These differences are within expected float32 precision bounds for different execution backends.

### What ONNX Enables

**Encoder swaps without Rust changes**: If DeBERTa becomes trainable (cloud GPU), or ModernBERT proves superior, the new model exports to ONNX and drops into the same `EventClassifier::load()` path. The Rust code doesn't change — it loads whatever ONNX model is at the expected path.

**Training framework independence**: The Rust inference code doesn't know whether the model was trained with HuggingFace Trainer, PyTorch Lightning, or a custom loop. ONNX is the contract.

**Runtime flexibility**: ONNX Runtime supports CPU, CUDA, CoreML, and other execution providers. The current deployment uses CPU. Future deployments can enable GPU acceleration by configuring the ort session builder — no model changes required.

---

## Future Path

### Revisit DeBERTa

When cloud GPU training is available (CUDA), DeBERTa-v3-small should be retrained and compared against DistilBERT on the Phase C.6 evaluation framework. If disentangled attention provides meaningful quality improvement on real prose, the size/latency tradeoff may be worthwhile.

### Evaluate ModernBERT

ModernBERT (if/when the ecosystem matures — tokenizer integration, ONNX export, HuggingFace Trainer support) could provide architectural improvements over the BERT family. Monitor the ecosystem before investing integration work.

### Quantization

The event classifier models are ~268MB each. INT8 quantization could reduce this to ~67MB each with minimal quality loss. ONNX Runtime supports post-training quantization. This is a deployment optimization, not a quality improvement — defer until deployment size becomes a constraint.

### All-Rust Training

The `burn` crate is the eventual path to all-Rust training and inference, eliminating the Python→ONNX→Rust pipeline. When `burn`'s transformer support and training infrastructure mature, it could replace the current split workflow. This is a long-term architectural goal, not an immediate priority — the current pipeline works and the Python training ecosystem is more capable today.
