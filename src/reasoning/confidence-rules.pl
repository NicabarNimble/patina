% Confidence Scoring Rules for Patina Persona System
% These rules are NOT suggestions - they are the law
% LLM must query these rules and obey the results

% ============================================================================
% CONFIDENCE LEVELS (Fixed Thresholds)
% ============================================================================

confidence_level(Weight, deprecated) :- Weight < 0.3.
confidence_level(Weight, low) :- Weight >= 0.3, Weight < 0.5.
confidence_level(Weight, uncertain) :- Weight >= 0.5, Weight < 0.7.
confidence_level(Weight, confident) :- Weight >= 0.7, Weight < 0.9.
confidence_level(Weight, very_confident) :- Weight >= 0.9.

% ============================================================================
% INITIAL CONFIDENCE (When Belief Is Created)
% ============================================================================

% New belief with no evidence starts at baseline
initial_confidence(0, 0.5).

% New belief with supporting evidence
initial_confidence(EvidenceCount, Confidence) :-
    EvidenceCount > 0,
    EvidenceCount =< 2,
    Confidence is 0.5 + (EvidenceCount * 0.15).  % 0.65 or 0.80

% New belief with strong evidence (3+ observations)
initial_confidence(EvidenceCount, Confidence) :-
    EvidenceCount >= 3,
    Confidence is min(0.85, 0.5 + (EvidenceCount * 0.1)).

% ============================================================================
% CONFIDENCE ADJUSTMENT (When New Evidence Appears)
% ============================================================================

% Strengthen: supporting evidence found
strengthen_confidence(CurrentWeight, SupportingCount, NewWeight) :-
    SupportingCount > 0,
    Increment is SupportingCount * 0.1,
    NewWeight is min(0.95, CurrentWeight + Increment).

% Weaken: contradicting evidence found
weaken_confidence(CurrentWeight, ContradictingCount, NewWeight) :-
    ContradictingCount > 0,
    Decrement is ContradictingCount * 0.15,
    NewWeight is max(0.3, CurrentWeight - Decrement).

% Adjust: mixed evidence (both supporting and contradicting)
adjust_confidence(CurrentWeight, Supporting, Contradicting, NewWeight) :-
    Supporting > 0,
    Contradicting > 0,
    NetEvidence is Supporting - Contradicting,
    (
        NetEvidence > 0 ->
            strengthen_confidence(CurrentWeight, NetEvidence, NewWeight)
        ;
        NetEvidence < 0 ->
            AbsNet is abs(NetEvidence),
            weaken_confidence(CurrentWeight, AbsNet, NewWeight)
        ;
        NewWeight = CurrentWeight  % Equal evidence, no change
    ).

% No change: no new evidence
adjust_confidence(CurrentWeight, 0, 0, CurrentWeight).

% ============================================================================
% REFINEMENT TRIGGERS (When to Ask User Questions)
% ============================================================================

% Belief needs refinement if confidence drops below threshold
needs_refinement(BeliefId, CurrentWeight, Reason) :-
    CurrentWeight < 0.6,
    Reason = 'confidence_below_threshold'.

% Belief needs refinement if contradicting evidence exists
needs_refinement(BeliefId, _, Reason) :-
    belief_has_contradiction(BeliefId),
    Reason = 'contradicting_evidence_found'.

% Belief needs refinement if old and never validated
needs_refinement(BeliefId, _, Reason) :-
    belief_age_days(BeliefId, Age),
    Age > 90,
    never_validated(BeliefId),
    Reason = 'stale_belief_never_validated'.

% Helper predicates (to be implemented from SQLite)
belief_has_contradiction(BeliefId) :-
    % Query: SELECT COUNT(*) FROM belief_observations WHERE belief_id = ? AND validates = 0
    false.  % Placeholder

belief_age_days(BeliefId, Days) :-
    % Query: SELECT julianday('now') - julianday(created_at) FROM beliefs WHERE id = ?
    false.  % Placeholder

never_validated(BeliefId) :-
    % Query: SELECT last_validated IS NULL FROM beliefs WHERE id = ?
    false.  % Placeholder

% ============================================================================
% TEMPORAL DECAY (Optional - beliefs lose confidence over time)
% ============================================================================

% Beliefs not seen in recent sessions lose confidence
temporal_weight(BaseWeight, DaysSinceValidation, AdjustedWeight) :-
    DaysSinceValidation > 180,
    DecayFactor is min(0.2, (DaysSinceValidation - 180) / 365 * 0.1),
    AdjustedWeight is max(0.4, BaseWeight - DecayFactor).

temporal_weight(BaseWeight, DaysSinceValidation, BaseWeight) :-
    DaysSinceValidation =< 180.

% ============================================================================
% CONFIDENCE BOUNDS (Hard Limits)
% ============================================================================

% Never exceed maximum confidence (leave room for doubt)
max_confidence(0.95).

% Never drop below minimum (keep deprecated beliefs discoverable)
min_confidence(0.3).

% Deprecated beliefs below this threshold are candidates for archival
archive_threshold(0.25).

% ============================================================================
% BELIEF EVOLUTION (When to Split/Merge Beliefs)
% ============================================================================

% Belief should split if evidence shows conditional pattern
should_split_belief(BeliefId, Reason) :-
    belief_has_mixed_evidence(BeliefId),
    evidence_suggests_condition(BeliefId, Condition),
    Reason = conditional_pattern_detected(Condition).

% Belief should merge if duplicates exist
should_merge_beliefs(Belief1, Belief2, Reason) :-
    beliefs_are_similar(Belief1, Belief2),
    beliefs_have_overlapping_evidence(Belief1, Belief2),
    Reason = 'duplicate_beliefs_detected'.

% ============================================================================
% QUERY INTERFACE (What LLM Can Ask)
% ============================================================================

% Query: What confidence should this new belief start with?
query_initial_confidence(EvidenceCount, Confidence) :-
    initial_confidence(EvidenceCount, Confidence).

% Query: How should confidence change given new evidence?
query_confidence_adjustment(CurrentWeight, Supporting, Contradicting, NewWeight) :-
    adjust_confidence(CurrentWeight, Supporting, Contradicting, NewWeight).

% Query: Does this belief need refinement?
query_needs_refinement(BeliefId, CurrentWeight, NeedsRefinement, Reason) :-
    (needs_refinement(BeliefId, CurrentWeight, Reason) ->
        NeedsRefinement = true
    ;
        NeedsRefinement = false, Reason = 'none').

% Query: What's the confidence level label?
query_confidence_level(Weight, Level) :-
    confidence_level(Weight, Level).

% Query: Should this belief be archived?
query_should_archive(Weight, ShouldArchive) :-
    archive_threshold(Threshold),
    (Weight < Threshold -> ShouldArchive = true ; ShouldArchive = false).

% ============================================================================
% VALIDATION RULES (Prevent Invalid States)
% ============================================================================

% Confidence must be within bounds
valid_confidence(Weight) :-
    Weight >= 0.0,
    Weight =< 1.0.

% Belief must have at least one piece of evidence if confidence > 0.5
valid_belief_state(BeliefId, Weight, EvidenceCount) :-
    (Weight > 0.5 -> EvidenceCount > 0 ; true).

% High confidence beliefs must have strong evidence
valid_high_confidence(Weight, EvidenceCount) :-
    Weight >= 0.9,
    EvidenceCount >= 3.

valid_high_confidence(Weight, _) :-
    Weight < 0.9.

% ============================================================================
% EXAMPLES (How LLM Should Use These Rules)
% ============================================================================

% Example 1: Creating new belief with 3 supporting patterns
% ?- query_initial_confidence(3, Confidence).
% Confidence = 0.85

% Example 2: Adjusting confidence after finding 2 supporting observations
% ?- query_confidence_adjustment(0.75, 2, 0, NewWeight).
% NewWeight = 0.95

% Example 3: Checking if belief needs refinement
% ?- query_needs_refinement(belief_123, 0.55, NeedsRefinement, Reason).
% NeedsRefinement = true
% Reason = 'confidence_below_threshold'

% Example 4: Mixed evidence (2 supporting, 1 contradicting)
% ?- query_confidence_adjustment(0.80, 2, 1, NewWeight).
% NewWeight = 0.90 (net +1 evidence, +0.1 confidence)
