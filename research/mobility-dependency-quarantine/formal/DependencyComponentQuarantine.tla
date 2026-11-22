---------------- MODULE DependencyComponentQuarantine ----------------
EXTENDS FiniteSets

CONSTANTS Actions, Edges, Seeds, InventoryComplete, GlobalDependency

ASSUME Actions # {}
ASSUME Seeds \subseteq Actions
ASSUME \A edge \in Edges : edge \in (Actions \X Actions)

OneStep(S) ==
  S \cup {b \in Actions : \E a \in S : <<a, b>> \in Edges \/ <<b, a>> \in Edges}

RECURSIVE Closure(_)
Closure(S) ==
  LET Expanded == OneStep(S)
  IN IF Expanded = S THEN S ELSE Closure(Expanded)

Decision ==
  IF ~InventoryComplete \/ GlobalDependency
  THEN Actions
  ELSE Closure(Seeds)

VARIABLE held

Init == held = Decision
Next == UNCHANGED held
Spec == Init /\ [][Next]_held

TypeOK == held \subseteq Actions
SeedsHeld == Seeds \subseteq held
NoUnderQuarantine == Decision \subseteq held
NoOverQuarantine == held \subseteq Decision
GlobalFallback == (~InventoryComplete \/ GlobalDependency) => held = Actions
LocalExactClosure == (InventoryComplete /\ ~GlobalDependency) => held = Closure(Seeds)

=============================================================================
