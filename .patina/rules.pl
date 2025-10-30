% Inference rules for Patina development knowledge
% Facts are in facts.pl - load both files together

% Pattern Evolution Rules
% ----------------------

% Pattern is recurring if observed in 2+ different sessions
recurring_pattern(Pattern) :-
    pattern_observed(S1, Pattern, _),
    pattern_observed(S2, Pattern, _),
    S1 \= S2.

% Pattern is mature if observed in 3+ sessions
% (simplified: if found in multiple places, it's recurring)
mature_pattern(Pattern) :-
    recurring_pattern(Pattern).

% Suggest promoting pattern from surface to core
promote_to_core(Pattern) :-
    mature_pattern(Pattern),
    pattern_observed(_, Pattern, Category),
    format('PROMOTE: ~w (~w) - observed in 3+ sessions~n', [Pattern, Category]).

% Pattern belongs to category
pattern_category(Pattern, Category) :-
    pattern_observed(_, Pattern, Category).

% Session Classification Rules
% ---------------------------

% Session produced code changes
productive_session(SessionId) :-
    session(SessionId, _, _, _, Commits, _),
    Commits > 0.

% Session was exploratory (no commits)
exploratory_session(SessionId) :-
    session(SessionId, _, exploration, _, 0, _).

% Session focused on specific category
session_focus(SessionId, Category) :-
    pattern_observed(SessionId, _, Category).

% Technology Discovery Rules
% -------------------------

% Technologies frequently used together
tech_pair(Tech1, Tech2) :-
    tech_used(S, Tech1, _),
    tech_used(S, Tech2, _),
    Tech1 \= Tech2.

% Technology used for specific purpose
tech_for_purpose(Technology, Purpose) :-
    tech_used(_, Technology, Purpose).

% Session used multiple technologies (integration work)
integration_session(SessionId) :-
    tech_used(SessionId, T1, _),
    tech_used(SessionId, T2, _),
    tech_used(SessionId, T3, _),
    T1 \= T2, T2 \= T3, T1 \= T3.

% Problem-Solution Mapping Rules
% ------------------------------

% Find solutions to similar problems
similar_challenge(Problem, Solution) :-
    challenge(_, Problem, Solution).

% Sessions that faced challenges in category
challenging_area(Category, SessionId) :-
    pattern_observed(SessionId, _, Category),
    challenge(SessionId, _, _).

% Decision Analysis Rules
% ----------------------

% Decisions made in specific session
session_decisions(SessionId, Choice, Rationale) :-
    decision(SessionId, Choice, Rationale).

% Philosophy-driven decisions
philosophical_decision(SessionId, Choice) :-
    decision(SessionId, Choice, Rationale),
    (sub_string(Rationale, _, _, _, "philosophy") ;
     sub_string(Rationale, _, _, _, "ethos") ;
     sub_string(Rationale, _, _, _, "principle")).

% Pragmatic decisions
pragmatic_decision(SessionId, Choice) :-
    decision(SessionId, Choice, Rationale),
    (sub_string(Rationale, _, _, _, "friction") ;
     sub_string(Rationale, _, _, _, "already") ;
     sub_string(Rationale, _, _, _, "faster")).

% Knowledge Graph Queries
% ----------------------

% Sessions working on same branch (workflow chain)
workflow_chain(SessionId1, SessionId2) :-
    session(SessionId1, _, _, Branch, _, _),
    session(SessionId2, _, _, Branch, _, _),
    SessionId1 \= SessionId2.

% Sessions in same time period (related work)
temporal_proximity(SessionId1, SessionId2) :-
    session(SessionId1, Date1, _, _, _, _),
    session(SessionId2, Date2, _, _, _, _),
    SessionId1 \= SessionId2,
    sub_string(Date1, 0, 7, _, Period),
    sub_string(Date2, 0, 7, _, Period).

% Pattern co-occurrence (patterns used together)
pattern_correlation(Pattern1, Pattern2) :-
    pattern_observed(S, Pattern1, _),
    pattern_observed(S, Pattern2, _),
    Pattern1 \= Pattern2.

% Cross-Domain Exploration (the unsolved problem)
% ---------------------------------------------

% What domains does this domain depend on?
depends_on_domain(SourceDomain, TargetDomain) :-
    domain_link(SourceDomain, TargetDomain, _).

% What is the relationship between domains?
domain_relationship(Source, Target, Relationship) :-
    domain_link(Source, Target, Relationship).

% High-Level Insights
% ------------------

% Security-focused sessions
security_session(SessionId) :-
    session_focus(SessionId, security).

% Architecture-focused sessions
architecture_session(SessionId) :-
    session_focus(SessionId, architecture).

% Most productive branch
productive_branch(Branch) :-
    session(_, _, _, Branch, Commits, _),
    Commits > 0.

% All patterns in a category (duplicates possible)
pattern_in_category(Pattern, Category) :-
    pattern_observed(_, Pattern, Category).
