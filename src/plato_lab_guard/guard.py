"""Lab guard with hypothesis gating and absolute quantifier detection."""

from dataclasses import dataclass
from enum import Enum

class GateResult(Enum):
    PASS = "pass"
    GATE = "gate"
    REJECT = "reject"

# Absolute quantifiers requiring evidence
ABSOLUTE_WORDS = frozenset([
    "all", "every", "always", "never", "none", "everyone", "everything",
    "nobody", "nothing", "everywhere", "nowhere", "completely", "entirely",
    "absolutely", "perfectly", "impossible", "certainly", "guaranteed",
    "proven", "definitive", "unanimous", "universal", "invariably"
])

# Vague causation patterns
VAGUE_CAUSATION = frozenset([
    "obviously", "clearly", "naturally", "of course", "it goes without saying",
    "as everyone knows", "self-evident", "needless to say", "it is well known"
])

@dataclass
class GuardReport:
    result: GateResult
    reason: str
    flags: list[str]
    confidence_impact: float

class LabGuard:
    def __init__(self, confidence_floor: float = 0.3, strict_mode: bool = False):
        self.confidence_floor = confidence_floor
        self.strict_mode = strict_mode

    def check(self, content: str, confidence: float = 0.5, domain: str = "") -> GuardReport:
        flags = []
        content_lower = content.lower()
        words = content_lower.split()

        # Gate 1: Absolute quantifier detection
        absolute_hits = []
        for i, w in enumerate(words):
            if w in ABSOLUTE_WORDS:
                absolute_hits.append(w)
        if absolute_hits:
            flags.append(f"absolute: {', '.join(absolute_hits)}")
            if confidence < 0.9:
                return GuardReport(GateResult.GATE,
                    f"Absolute claim ({', '.join(absolute_hits[:3])}) needs confidence >= 0.9",
                    flags, -0.2)

        # Gate 2: Vague causation
        for phrase in VAGUE_CAUSATION:
            if phrase in content_lower:
                flags.append(f"vague_causation: {phrase[:20]}")
                return GuardReport(GateResult.GATE,
                    f"Vague causation detected: '{phrase[:30]}'", flags, -0.1)

        # Gate 3: Confidence floor
        if confidence < self.confidence_floor:
            flags.append("low_confidence")
            return GuardReport(GateResult.REJECT,
                f"Confidence {confidence:.2f} below floor {self.confidence_floor}",
                flags, -0.3)

        # Gate 4: Content length
        if len(content) < 10:
            flags.append("too_short")
            return GuardReport(GateResult.REJECT, "Content below 10 chars", flags, -0.5)

        # Gate 5: Domain-specific checks
        if domain == "research" and len(content) < 50:
            flags.append("research_too_short")
            if self.strict_mode:
                return GuardReport(GateResult.GATE, "Research tile too short", flags, -0.1)

        return GuardReport(GateResult.PASS, "All gates passed", flags, 0.0)

    def check_batch(self, items: list[dict]) -> list[GuardReport]:
        return [self.check(i.get("content", ""), i.get("confidence", 0.5), i.get("domain", ""))
                for i in items]

    @property
    def stats(self) -> dict:
        return {"confidence_floor": self.confidence_floor, "strict_mode": self.strict_mode,
                "absolute_words": len(ABSOLUTE_WORDS), "vague_patterns": len(VAGUE_CAUSATION)}
