% Validation Rules for Belief System
% These rules reason over observations loaded from semantic search
% to validate beliefs before insertion into the knowledge base

% ============================================================================
% EVIDENCE COUNTING (Weighted by Similarity and Reliability)
% ============================================================================

% Count strong evidence (high similarity + high reliability)
count_strong_evidence(Count) :-
    findall(Weight,
        (observation(_, _, _, Sim, Rel, _),
         Sim >= 0.70,
         Rel >= 0.70,
         Weight is Sim * Rel),
        Weights),
    length(Weights, Count).

% Calculate weighted evidence score
weighted_evidence_score(Score) :-
    findall(Weight,
        (observation(_, _, _, Sim, Rel, _),
         Sim >= 0.50,  % Only count medium+ similarity
         Weight is Sim * Rel),
        Weights),
    sum_list(Weights, Score).

% ============================================================================
% CONTRADICTION DETECTION
% ============================================================================

% Find observations that might contradict each other
% (high similarity to query but with conflicting content patterns)
find_contradictions(Contradictions) :-
    findall([Id1, Content1, Id2, Content2],
        (observation(Id1, _, Content1, Sim1, _, _),
         observation(Id2, _, Content2, Sim2, _, _),
         Id1 \= Id2,
         Sim1 >= 0.70,
         Sim2 >= 0.70,
         possibly_contradictory(Content1, Content2)),
        Contradictions).

% Heuristic: content might be contradictory if both are high similarity
% In practice, LLM can detect semantic contradictions
possibly_contradictory(Content1, Content2) :-
    % Placeholder - actual contradiction detection would use semantic analysis
    % For now, just check they're different
    Content1 \= Content2.

% ============================================================================
% BELIEF VALIDATION
% ============================================================================

% Validate a belief based on loaded observations
% Returns: valid/invalid and reason
validate_belief(Valid, Reason) :-
    % Check for contradictions
    find_contradictions(Contradictions),
    (   Contradictions = []
    ->  % No contradictions, check evidence strength
        weighted_evidence_score(Score),
        count_strong_evidence(StrongCount),
        (   Score >= 5.0, StrongCount >= 2
        ->  Valid = true,
            Reason = 'sufficient_strong_evidence'
        ;   Score >= 3.0
        ->  Valid = true,
            Reason = 'adequate_evidence'
        ;   Valid = false,
            Reason = 'weak_evidence')
    ;   % Contradictions found
        length(Contradictions, ContraCount),
        weighted_evidence_score(Score),
        (   Score >= 8.0, ContraCount =< 1
        ->  Valid = true,
            Reason = 'strong_evidence_despite_contradiction'
        ;   Valid = false,
            format(atom(Reason), 'contradictions_found: ~w', [ContraCount]))
    ).

% ============================================================================
% SOURCE RELIABILITY CHECKS
% ============================================================================

% Check if we have observations from multiple independent sources
has_diverse_sources(Diverse) :-
    findall(Source, observation(_, _, _, _, _, Source), Sources),
    list_to_set(Sources, UniqueSet),
    length(UniqueSet, Count),
    (Count >= 2 -> Diverse = true ; Diverse = false).

% Get source distribution for debugging
source_distribution(Distribution) :-
    findall(Source-Count,
        (observation(_, _, _, _, _, Source),
         findall(S, observation(_, _, _, _, _, S), AllSources),
         findall(X, (member(X, AllSources), X = Source), Matches),
         length(Matches, Count)),
        Distribution).

% ============================================================================
% EVIDENCE QUALITY METRICS
% ============================================================================

% Calculate average reliability of observations
average_reliability(AvgRel) :-
    findall(Rel, observation(_, _, _, _, Rel, _), Reliabilities),
    (   Reliabilities = []
    ->  AvgRel = 0.0
    ;   sum_list(Reliabilities, Sum),
        length(Reliabilities, Count),
        AvgRel is Sum / Count
    ).

% Calculate average similarity of observations
average_similarity(AvgSim) :-
    findall(Sim, observation(_, _, _, Sim, _, _), Similarities),
    (   Similarities = []
    ->  AvgSim = 0.0
    ;   sum_list(Similarities, Sum),
        length(Similarities, Count),
        AvgSim is Sum / Count
    ).

% ============================================================================
% QUERY INTERFACE
% ============================================================================

% Main query for belief validation
query_validate_belief(Valid, Reason, Metrics) :-
    validate_belief(Valid, Reason),
    weighted_evidence_score(Score),
    count_strong_evidence(StrongCount),
    has_diverse_sources(Diverse),
    average_reliability(AvgRel),
    average_similarity(AvgSim),
    Metrics = metrics(Score, StrongCount, Diverse, AvgRel, AvgSim).
