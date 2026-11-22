---------------- MODULE DependencyComponentQuarantine ----------------
EXTENDS FiniteSets

CONSTANTS Actions, Edges, Seeds, InventoryComplete, GlobalDependency

ASSUME Actions # {}
ASSUME Seeds \subseteq Actions
ASSUME Edges \subseteq SUBSET Actions
ASSUME \A edge \in Edges : Cardinality(edge) >= 2

Successors(S) ==
    {b \in Actions : \E edge \in Edges : b \in edge /\ edge \cap S # {}}

RECURSIVE Closure(_)
Closure(S) ==
    LET Expanded == S \cup Successors(S)
    IN IF Expanded = S THEN S ELSE Closure(Expanded)

Fallback == ~InventoryComplete \/ GlobalDependency

VARIABLES quarantined, frontier, outside, done, global_hold
vars == <<quarantined, frontier, outside, done, global_hold>>

Init ==
    IF Fallback
    THEN
        /\ quarantined = Actions
        /\ frontier = {}
        /\ outside = {}
        /\ done = TRUE
        /\ global_hold = TRUE
    ELSE
        /\ quarantined = Seeds
        /\ frontier = Seeds
        /\ outside = {}
        /\ done = FALSE
        /\ global_hold = FALSE

Next ==
    \/ /\ ~global_hold
       /\ ~done
       /\ frontier # {}
       /\ LET added == Successors(frontier) \ quarantined
          IN
             /\ quarantined' = quarantined \cup added
             /\ frontier' = added
             /\ outside' = outside
             /\ done' = FALSE
             /\ global_hold' = FALSE
    \/ /\ ~global_hold
       /\ ~done
       /\ frontier = {}
       /\ quarantined' = quarantined
       /\ frontier' = {}
       /\ outside' = Actions \ quarantined
       /\ done' = TRUE
       /\ global_hold' = FALSE
    \/ /\ (global_hold \/ done)
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ quarantined \subseteq Actions
    /\ frontier \subseteq Actions
    /\ outside \subseteq Actions
    /\ done \in BOOLEAN
    /\ global_hold \in BOOLEAN

SeedsAlwaysHeld == Seeds \subseteq quarantined
FrontierIsHeld == frontier \subseteq quarantined
NoPrematureOutside == ~done => outside = {}
NoUnderQuarantine == outside \cap Closure(Seeds) = {}
ExactLocalClosure ==
    (done /\ ~global_hold)
    => /\ quarantined = Closure(Seeds)
       /\ outside = Actions \ Closure(Seeds)
GlobalFallback ==
    global_hold => /\ quarantined = Actions
                  /\ outside = {}

=============================================================================
