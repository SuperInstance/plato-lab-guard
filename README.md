# plato-lab-guard

Unfakeable constraint lab — Achievement Loss scoring prevents cherry-picked results.

## What This Does

Every hypothesis in the constraint theory lab runs through experiments. Results are scored by **Achievement Loss** — did the agent actually *learn*, or just memorize? Only hypotheses that pass the Achievement Loss threshold get marked "confirmed."

This makes the lab unfakeable. You can't game the system by cherry-picking favorable results.

## Achievement Loss

```
loss = 1.0 - (comprehension × generalization × retention)
```

- **Comprehension** (0.0-1.0): Did the agent understand the constraint?
- **Generalization** (0.0-1.0): Does it work on unseen data?
- **Retention** (0.0-1.0): Does it persist after intervening tasks?

Low loss = real learning. High loss = memorization or noise.

## Quick Start

```rust
use plato_lab_guard::{LabGuard, Hypothesis, ExperimentResult};

let mut guard = LabGuard::new();
let hyp = Hypothesis::new("snap-precision", "Snapping to Pythagorean coordinates produces zero drift", 0.01);
guard.submit(hyp);

let result = ExperimentResult {
    hypothesis_id: "snap-precision".to_string(),
    comprehension: 0.92,
    generalization: 0.88,
    retention: 0.85,
    raw_accuracy: 0.99, // cherry-picked metric
};

let verdict = guard.evaluate(&result);
// Achievement loss = 1 - (0.92 × 0.88 × 0.85) = 0.312
// Verdict: CONFIRMED (loss 0.31 < threshold 0.4)
// BUT raw_accuracy 0.99 alone would be misleading
```

## The Four Gates

Every hypothesis passes through four gates before evaluation:
1. **Well-formed** — claim + conditions + threshold required
2. **Falsifiable** — no absolute words (always/never/all)
3. **Novel** — not already tested
4. **Bounded** — positive numeric threshold

## Integration

- `ct-lab`: Hypothesis submission and validation room
- `plato-achievement`: Achievement Loss metric (source crate)
- `jepa-perception-lab`: CUDA experiment runner
- `constraint-theory-core`: Mathematical foundation

## License

MIT
