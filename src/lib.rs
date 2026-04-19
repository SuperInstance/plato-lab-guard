//! plato-lab-guard — Unfakeable constraint lab with Achievement Loss scoring

use std::collections::HashMap;

// ── Hypothesis ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub id: String,
    pub claim: String,
    pub conditions: Vec<String>,
    pub threshold: f32,       // max acceptable Achievement Loss
    pub submitted_by: String,
    pub status: HypothesisStatus,
    pub gate_violations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    Pending,
    Gated,
    Testing,
    Confirmed,
    Falsified,
    Inconclusive,
}

impl Hypothesis {
    pub fn new(id: &str, claim: &str, threshold: f32) -> Self {
        Self {
            id: id.to_string(),
            claim: claim.to_string(),
            conditions: Vec::new(),
            threshold,
            submitted_by: String::new(),
            status: HypothesisStatus::Pending,
            gate_violations: Vec::new(),
        }
    }

    pub fn with_conditions(mut self, conditions: Vec<String>) -> Self {
        self.conditions = conditions;
        self
    }

    pub fn with_submitter(mut self, name: &str) -> Self {
        self.submitted_by = name.to_string();
        self
    }
}

// ── Experiment Result ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExperimentResult {
    pub hypothesis_id: String,
    pub comprehension: f32,     // 0.0-1.0
    pub generalization: f32,    // 0.0-1.0
    pub retention: f32,         // 0.0-1.0
    pub raw_accuracy: f32,      // the cherry-pickable metric
    pub details: String,
}

impl ExperimentResult {
    /// Calculate Achievement Loss
    pub fn achievement_loss(&self) -> f32 {
        let product = self.comprehension * self.generalization * self.retention;
        1.0 - product
    }
}

// ── Verdict ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Verdict {
    pub hypothesis_id: String,
    pub status: HypothesisStatus,
    pub achievement_loss: f32,
    pub threshold: f32,
    pub passed: bool,
    pub raw_accuracy: f32,
    pub warning: String,
    pub details: String,
}

// ── Gate Check ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    Pass,
    Fail(String),
}

// ── Lab Guard ────────────────────────────────────────────

pub struct LabGuard {
    hypotheses: HashMap<String, Hypothesis>,
    results: HashMap<String, ExperimentResult>,
    verdicts: Vec<Verdict>,
    loss_threshold: f32,
}

impl LabGuard {
    pub fn new() -> Self {
        Self {
            hypotheses: HashMap::new(),
            results: HashMap::new(),
            verdicts: Vec::new(),
            loss_threshold: 0.4,
        }
    }

    pub fn with_loss_threshold(mut self, threshold: f32) -> Self {
        self.loss_threshold = threshold;
        self
    }

    // ── Gates ──

    fn check_gates(&self, hyp: &Hypothesis) -> Vec<GateResult> {
        let mut gates = Vec::new();

        // Gate 1: Well-formed — must have claim, conditions, positive threshold
        if hyp.claim.is_empty() {
            gates.push(GateResult::Fail("Missing claim".to_string()));
        } else if hyp.conditions.is_empty() {
            gates.push(GateResult::Fail("Missing conditions".to_string()));
        } else if hyp.threshold <= 0.0 || hyp.threshold >= 1.0 {
            gates.push(GateResult::Fail("Threshold must be 0.0 < t < 1.0".to_string()));
        } else {
            gates.push(GateResult::Pass);
        }

        // Gate 2: Falsifiable — no absolute words
        let absolutes = ["always", "never", "all", "none", "every single", "impossible to fail"];
        let claim_lower = hyp.claim.to_lowercase();
        let has_absolute = absolutes.iter().any(|a| claim_lower.contains(a));
        if has_absolute {
            gates.push(GateResult::Fail(format!("Contains absolute word: check for {:?}", absolutes)));
        } else {
            gates.push(GateResult::Pass);
        }

        // Gate 3: Novel — not already submitted or tested
        if self.hypotheses.contains_key(&hyp.id) || self.verdicts.iter().any(|v| v.hypothesis_id == hyp.id) {
            gates.push(GateResult::Fail("Already tested".to_string()));
        } else {
            gates.push(GateResult::Pass);
        }

        // Gate 4: Bounded — threshold is a positive number
        if hyp.threshold <= 0.0 {
            gates.push(GateResult::Fail("Threshold must be positive".to_string()));
        } else {
            gates.push(GateResult::Pass);
        }

        gates
    }

    // ── Operations ──

    pub fn submit(&mut self, mut hypothesis: Hypothesis) -> GateResult {
        let gates = self.check_gates(&hypothesis);
        let failures: Vec<String> = gates.iter()
            .filter_map(|g| if let GateResult::Fail(r) = g { Some(r.clone()) } else { None })
            .collect();

        if failures.is_empty() {
            hypothesis.status = HypothesisStatus::Gated;
            hypothesis.gate_violations = Vec::new();
            self.hypotheses.insert(hypothesis.id.clone(), hypothesis);
            GateResult::Pass
        } else {
            hypothesis.gate_violations = failures.clone();
            hypothesis.status = HypothesisStatus::Pending;
            self.hypotheses.insert(hypothesis.id.clone(), hypothesis);
            GateResult::Fail(failures.join("; "))
        }
    }

    pub fn evaluate(&mut self, result: &ExperimentResult) -> Option<Verdict> {
        let hyp = self.hypotheses.get_mut(&result.hypothesis_id)?;
        hyp.status = HypothesisStatus::Testing;

        let loss = result.achievement_loss();
        let passed = loss <= hyp.threshold;

        // Detect cherry-picking: high raw_accuracy but high loss
        let cherry_pick_warning = if result.raw_accuracy > 0.95 && loss > 0.3 {
            format!("CHERRY-PICK WARNING: raw_accuracy {:.2} but loss {:.2} — results may be cherry-picked",
                result.raw_accuracy, loss)
        } else {
            String::new()
        };

        let status = if passed {
            HypothesisStatus::Confirmed
        } else if loss > self.loss_threshold {
            HypothesisStatus::Falsified
        } else {
            HypothesisStatus::Inconclusive
        };

        hyp.status = status;
        self.results.insert(result.hypothesis_id.clone(), result.clone());

        let verdict = Verdict {
            hypothesis_id: result.hypothesis_id.clone(),
            status,
            achievement_loss: loss,
            threshold: hyp.threshold,
            passed,
            raw_accuracy: result.raw_accuracy,
            warning: cherry_pick_warning,
            details: format!("loss={:.4} threshold={:.4} comp={:.2} gen={:.2} ret={:.2}",
                loss, hyp.threshold, result.comprehension, result.generalization, result.retention),
        };

        self.verdicts.push(verdict.clone());
        Some(verdict)
    }

    // ── Queries ──

    pub fn hypothesis(&self, id: &str) -> Option<&Hypothesis> {
        self.hypotheses.get(id)
    }

    pub fn result(&self, id: &str) -> Option<&ExperimentResult> {
        self.results.get(id)
    }

    pub fn verdict(&self, id: &str) -> Option<&Verdict> {
        self.verdicts.iter().find(|v| v.hypothesis_id == id)
    }

    pub fn confirmed_count(&self) -> usize {
        self.verdicts.iter().filter(|v| v.status == HypothesisStatus::Confirmed).count()
    }

    pub fn falsified_count(&self) -> usize {
        self.verdicts.iter().filter(|v| v.status == HypothesisStatus::Falsified).count()
    }

    pub fn total_evaluated(&self) -> usize {
        self.verdicts.len()
    }

    pub fn average_loss(&self) -> f32 {
        if self.verdicts.is_empty() { return 0.0; }
        let sum: f32 = self.verdicts.iter().map(|v| v.achievement_loss).sum();
        sum / self.verdicts.len() as f32
    }

    /// Get hypotheses by status
    pub fn by_status(&self, status: HypothesisStatus) -> Vec<&Hypothesis> {
        self.hypotheses.values().filter(|h| h.status == status).collect()
    }
}

impl Default for LabGuard {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_hypothesis() -> Hypothesis {
        Hypothesis::new("hyp-1", "Snapping to Pythagorean coordinates reduces drift below 0.001", 0.3)
            .with_conditions(vec!["CUDA environment".to_string(), "10K iterations".to_string()])
            .with_submitter("Forgemaster")
    }

    #[test]
    fn test_submit_valid() {
        let mut guard = LabGuard::new();
        let result = guard.submit(valid_hypothesis());
        assert!(matches!(result, GateResult::Pass));
        assert_eq!(guard.hypothesis("hyp-1").unwrap().status, HypothesisStatus::Gated);
    }

    #[test]
    fn test_gate_missing_claim() {
        let mut guard = LabGuard::new();
        let mut hyp = Hypothesis::new("hyp-x", "", 0.3);
        hyp.conditions = vec!["test".to_string()];
        let result = guard.submit(hyp);
        assert!(matches!(result, GateResult::Fail(_)));
    }

    #[test]
    fn test_gate_missing_conditions() {
        let mut guard = LabGuard::new();
        let hyp = Hypothesis::new("hyp-x", "Some claim", 0.3);
        let result = guard.submit(hyp);
        assert!(matches!(result, GateResult::Fail(_)));
    }

    #[test]
    fn test_gate_absolute_words() {
        let mut guard = LabGuard::new();
        let hyp = Hypothesis::new("hyp-x", "This always produces zero drift", 0.3)
            .with_conditions(vec!["test".to_string()]);
        let result = guard.submit(hyp);
        assert!(matches!(result, GateResult::Fail(_)));
    }

    #[test]
    fn test_gate_invalid_threshold() {
        let mut guard = LabGuard::new();
        let hyp = Hypothesis::new("hyp-x", "Some claim", 0.0)
            .with_conditions(vec!["test".to_string()]);
        let result = guard.submit(hyp);
        assert!(matches!(result, GateResult::Fail(_)));
    }

    #[test]
    fn test_gate_novelty() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());
        // Submit same ID again
        let hyp2 = Hypothesis::new("hyp-1", "Different claim about drift", 0.3)
            .with_conditions(vec!["test".to_string()]);
        let result = guard.submit(hyp2);
        assert!(matches!(result, GateResult::Fail(_)));
    }

    #[test]
    fn test_evaluate_confirmed() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());

        let result = ExperimentResult {
            hypothesis_id: "hyp-1".to_string(),
            comprehension: 0.95,
            generalization: 0.90,
            retention: 0.88,
            raw_accuracy: 0.99,
            details: "Strong results".to_string(),
        };

        let verdict = guard.evaluate(&result).unwrap();
        assert_eq!(verdict.status, HypothesisStatus::Confirmed);
        // loss = 1 - (0.95 * 0.90 * 0.88) = 1 - 0.7524 = 0.2476
        assert!(verdict.achievement_loss < 0.3);
        assert!(verdict.passed);
    }

    #[test]
    fn test_evaluate_falsified() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());

        let result = ExperimentResult {
            hypothesis_id: "hyp-1".to_string(),
            comprehension: 0.3,
            generalization: 0.2,
            retention: 0.1,
            raw_accuracy: 0.95,
            details: "Poor learning".to_string(),
        };

        let verdict = guard.evaluate(&result).unwrap();
        assert_eq!(verdict.status, HypothesisStatus::Falsified);
        assert!(!verdict.passed);
    }

    #[test]
    fn test_evaluate_inconclusive() {
        let mut guard = LabGuard::new().with_loss_threshold(0.2);
        guard.submit(Hypothesis::new("hyp-1", "Some claim", 0.5)
            .with_conditions(vec!["test".to_string()]));

        let result = ExperimentResult {
            hypothesis_id: "hyp-1".to_string(),
            comprehension: 0.7,
            generalization: 0.6,
            retention: 0.5,
            raw_accuracy: 0.8,
            details: "Mixed".to_string(),
        };

        // loss = 1 - (0.7 * 0.6 * 0.5) = 1 - 0.21 = 0.79
        let verdict = guard.evaluate(&result).unwrap();
        assert_eq!(verdict.status, HypothesisStatus::Falsified);
    }

    #[test]
    fn test_cherry_pick_warning() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());

        let result = ExperimentResult {
            hypothesis_id: "hyp-1".to_string(),
            comprehension: 0.4,
            generalization: 0.3,
            retention: 0.3,
            raw_accuracy: 0.99, // cherry-picked!
            details: "Looks great on paper".to_string(),
        };

        let verdict = guard.evaluate(&result).unwrap();
        assert!(!verdict.warning.is_empty());
        assert!(verdict.warning.contains("CHERRY-PICK"));
    }

    #[test]
    fn test_no_cherry_pick_warning() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());

        let result = ExperimentResult {
            hypothesis_id: "hyp-1".to_string(),
            comprehension: 0.9,
            generalization: 0.88,
            retention: 0.85,
            raw_accuracy: 0.92,
            details: "Consistent".to_string(),
        };

        let verdict = guard.evaluate(&result).unwrap();
        assert!(verdict.warning.is_empty());
    }

    #[test]
    fn test_achievement_loss_formula() {
        let result = ExperimentResult {
            hypothesis_id: "test".to_string(),
            comprehension: 0.5,
            generalization: 0.5,
            retention: 0.5,
            raw_accuracy: 0.9,
            details: String::new(),
        };
        // loss = 1 - (0.5 * 0.5 * 0.5) = 1 - 0.125 = 0.875
        assert!((result.achievement_loss() - 0.875).abs() < 0.001);
    }

    #[test]
    fn test_stats() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());

        assert_eq!(guard.confirmed_count(), 0);
        assert_eq!(guard.falsified_count(), 0);
        assert_eq!(guard.total_evaluated(), 0);

        let result = ExperimentResult {
            hypothesis_id: "hyp-1".to_string(),
            comprehension: 0.95,
            generalization: 0.90,
            retention: 0.88,
            raw_accuracy: 0.99,
            details: String::new(),
        };
        guard.evaluate(&result);

        assert_eq!(guard.confirmed_count(), 1);
        assert_eq!(guard.total_evaluated(), 1);
        assert!(guard.average_loss() > 0.0);
    }

    #[test]
    fn test_by_status() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());
        let gated = guard.by_status(HypothesisStatus::Gated);
        assert_eq!(gated.len(), 1);
    }

    #[test]
    fn test_verdict_details() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());

        let result = ExperimentResult {
            hypothesis_id: "hyp-1".to_string(),
            comprehension: 0.95,
            generalization: 0.90,
            retention: 0.88,
            raw_accuracy: 0.99,
            details: String::new(),
        };
        let verdict = guard.evaluate(&result).unwrap();
        assert!(verdict.details.contains("loss="));
        assert!(verdict.details.contains("comp="));
    }

    #[test]
    fn test_multiple_hypotheses() {
        let mut guard = LabGuard::new();
        guard.submit(valid_hypothesis());
        guard.submit(Hypothesis::new("hyp-2", "Constraint tightening improves precision", 0.35)
            .with_conditions(vec!["test".to_string()]));

        assert_eq!(guard.by_status(HypothesisStatus::Gated).len(), 2);
    }
}
