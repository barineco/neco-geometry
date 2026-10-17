import NecoAlgebraVerify.Soundness
import Mathlib.Algebra.MonoidAlgebra.Basic
import Mathlib.Data.List.Sort

set_option autoImplicit false

namespace NecoGeometryVerify

structure RegionTerm where
  coefficient : Rat
  exponentX : Nat
  exponentY : Nat
  exponentZ : Nat
  deriving Repr, BEq, DecidableEq

namespace RegionPolynomial

private def precedes (left right : RegionTerm) : Bool :=
  left.exponentX < right.exponentX ||
    (left.exponentX == right.exponentX &&
      (left.exponentY < right.exponentY ||
        (left.exponentY == right.exponentY && left.exponentZ < right.exponentZ)))

private def mergeTerm (term : RegionTerm) : List RegionTerm → List RegionTerm
  | [] => if term.coefficient == 0 then [] else [term]
  | current :: remaining =>
      if current.exponentX == term.exponentX && current.exponentY == term.exponentY &&
          current.exponentZ == term.exponentZ then
        let merged : RegionTerm :=
          { coefficient := current.coefficient + term.coefficient
            exponentX := current.exponentX
            exponentY := current.exponentY
            exponentZ := current.exponentZ }
        if merged.coefficient == 0 then remaining else merged :: remaining
      else
        current :: mergeTerm term remaining

private def insertSorted (term : RegionTerm) : List RegionTerm → List RegionTerm
  | [] => [term]
  | current :: remaining =>
      if precedes term current then term :: current :: remaining
      else current :: insertSorted term remaining

def normalize (rawTerms : List RegionTerm) : List RegionTerm :=
  let merged := rawTerms.foldl (fun terms term => mergeTerm term terms) []
  merged.foldl (fun terms term => insertSorted term terms) []

def monomial (term : RegionTerm) (x y z : Rat) : Rat :=
  term.coefficient * x ^ term.exponentX * y ^ term.exponentY * z ^ term.exponentZ

def eval (terms : List RegionTerm) (x y z : Rat) : Rat :=
  terms.foldl (fun total term => total + monomial term x y z) 0

def rawScale (scalar : Rat) (poly : List RegionTerm) : List RegionTerm :=
  poly.map fun term => { term with coefficient := scalar * term.coefficient }

def scale (scalar : Rat) (terms : List RegionTerm) : List RegionTerm :=
  normalize (rawScale scalar terms)

def neg (poly : List RegionTerm) : List RegionTerm := rawScale (-1) poly

def add (left right : List RegionTerm) : List RegionTerm :=
  normalize (left ++ right)

def sub (left right : List RegionTerm) : List RegionTerm :=
  normalize (left ++ neg right)

private def mulTerm (left right : RegionTerm) : RegionTerm :=
  { coefficient := left.coefficient * right.coefficient
    exponentX := left.exponentX + right.exponentX
    exponentY := left.exponentY + right.exponentY
    exponentZ := left.exponentZ + right.exponentZ }

def mul (left right : List RegionTerm) : List RegionTerm :=
  normalize
    (left.foldl (fun acc lt => acc ++ right.map (mulTerm lt)) [])

private theorem foldl_append_map (left right : List RegionTerm) :
    ∀ acc, left.foldl (fun acc lt => acc ++ right.map (mulTerm lt)) acc =
      acc ++ left.foldr (fun lt acc => right.map (mulTerm lt) ++ acc) [] := by
  induction left with
  | nil => intro acc; simp
  | cons lt rest ih =>
      intro acc
      simp only [List.foldl_cons, List.foldr_cons]
      rw [ih]
      exact List.append_assoc _ _ _

theorem mul_eq_foldr (left right : List RegionTerm) :
    mul left right =
      normalize (left.foldr (fun lt acc => right.map (mulTerm lt) ++ acc) []) := by
  unfold mul
  rw [foldl_append_map]
  rfl

def pow : List RegionTerm → Nat → List RegionTerm
  | _, 0 => [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 0 }]
  | poly, exponent + 1 => mul (pow poly exponent) poly

def variablePolynomial (axis : Nat) : List RegionTerm :=
  match axis with
  | 0 => [{ coefficient := 1, exponentX := 1, exponentY := 0, exponentZ := 0 }]
  | 1 => [{ coefficient := 1, exponentX := 0, exponentY := 1, exponentZ := 0 }]
  | _ => [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 1 }]

def constant (value : Rat) : List RegionTerm :=
  if value == 0 then [] else
    [{ coefficient := value, exponentX := 0, exponentY := 0, exponentZ := 0 }]

private theorem foldl_add (g : RegionTerm → Rat) (terms : List RegionTerm) :
    ∀ acc, terms.foldl (fun total term => total + g term) acc =
      acc + terms.foldl (fun total term => total + g term) 0 := by
  induction terms with
  | nil => intro acc; simp
  | cons term remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih (acc + g term), ih (0 + g term)]
      ring

private theorem eval_foldl (terms : List RegionTerm) (acc x y z : Rat) :
    terms.foldl (fun total term => total + monomial term x y z) acc =
      acc + eval terms x y z :=
  foldl_add _ terms acc

theorem eval_append (left right : List RegionTerm) (x y z : Rat) :
    eval (left ++ right) x y z = eval left x y z + eval right x y z := by
  unfold eval
  rw [List.foldl_append, eval_foldl]
  simp [eval_foldl]

theorem monomial_add_term (left right : RegionTerm) (x y z : Rat)
    (sameX : left.exponentX = right.exponentX)
    (sameY : left.exponentY = right.exponentY)
    (sameZ : left.exponentZ = right.exponentZ) :
    monomial ⟨left.coefficient + right.coefficient, left.exponentX,
        left.exponentY, left.exponentZ⟩ x y z =
      monomial left x y z + monomial right x y z := by
  simp only [monomial, sameX, sameY, sameZ]
  ring

private theorem mergeTerm_eval (term : RegionTerm) (acc : List RegionTerm) (x y z : Rat) :
    eval (mergeTerm term acc) x y z =
      monomial term x y z + eval acc x y z := by
  induction acc with
  | nil =>
      unfold mergeTerm
      split
      · rename_i hzero
        have hz : term.coefficient = 0 := beq_iff_eq.mp hzero
        simp [eval, monomial, hz]
      · simp [eval, List.foldl_cons]
  | cons current remaining ih =>
      unfold mergeTerm
      split
      · rename_i hsame
        have ⟨hxy, hz'⟩ := Bool.and_eq_true _ _ |>.mp hsame
        have ⟨hx, hy⟩ := Bool.and_eq_true _ _ |>.mp hxy
        have hx' : current.exponentX = term.exponentX := beq_iff_eq.mp hx
        have hy' : current.exponentY = term.exponentY := beq_iff_eq.mp hy
        have hz'' : current.exponentZ = term.exponentZ := beq_iff_eq.mp hz'
        have hmono := monomial_add_term current term x y z hx' hy' hz''
        dsimp only
        split
        · rename_i hm
          have hcoef : current.coefficient + term.coefficient = 0 :=
            beq_iff_eq.mp hm
          have hzero : monomial current x y z + monomial term x y z = 0 := by
            rw [← hmono]
            simp [monomial, hcoef]
          simp only [eval, List.foldl_cons]
          simp only [eval_foldl]
          linarith
        · simp only [eval, List.foldl_cons]
          simp only [eval_foldl]
          rw [hmono]
          ring
      · simp only [eval, List.foldl_cons]
        simp only [eval_foldl]
        rw [ih]
        ring

private theorem insertSorted_eval (term : RegionTerm) (acc : List RegionTerm)
    (x y z : Rat) :
    eval (insertSorted term acc) x y z =
      monomial term x y z + eval acc x y z := by
  induction acc with
  | nil => simp [insertSorted, eval, List.foldl_cons]
  | cons current remaining ih =>
      unfold insertSorted
      split
      · simp only [eval, List.foldl_cons]
        simp only [eval_foldl]
        ring
      · simp only [eval, List.foldl_cons]
        simp only [eval_foldl]
        rw [ih]
        ring

private theorem mergeAll_eval (rawTerms : List RegionTerm) (x y z : Rat) :
    ∀ acc, eval (rawTerms.foldl (fun terms term => mergeTerm term terms) acc) x y z =
      eval acc x y z + eval rawTerms x y z := by
  induction rawTerms with
  | nil => intro acc; simp [eval]
  | cons term remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih, mergeTerm_eval]
      simp only [eval, List.foldl_cons]
      simp only [eval_foldl]
      ring

private theorem sortAll_eval (terms : List RegionTerm) (x y z : Rat) :
    ∀ acc, eval (terms.foldl (fun acc' term => insertSorted term acc') acc) x y z =
      eval acc x y z + eval terms x y z := by
  induction terms with
  | nil => intro acc; simp [eval]
  | cons term remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih, insertSorted_eval]
      simp only [eval, List.foldl_cons]
      simp only [eval_foldl]
      ring

theorem normalize_eval (terms : List RegionTerm) (x y z : Rat) :
    eval (normalize terms) x y z = eval terms x y z := by
  unfold normalize
  rw [sortAll_eval, mergeAll_eval]
  simp [eval]

theorem add_eval (left right : List RegionTerm) (x y z : Rat) :
    eval (add left right) x y z =
      eval left x y z + eval right x y z := by
  unfold add
  rw [normalize_eval, eval_append]

private theorem rawScale_foldl (scalar : Rat) (poly : List RegionTerm) (acc x y z : Rat) :
    poly.foldl (fun total term => total +
        monomial { term with coefficient := scalar * term.coefficient } x y z) acc =
      acc + scalar *
        poly.foldl (fun total term => total + monomial term x y z) 0 := by
  induction poly generalizing acc with
  | nil => simp
  | cons term remaining ih =>
      simp only [List.foldl_cons]
      rw [ih (acc + monomial { term with coefficient := scalar * term.coefficient }
        x y z),
        foldl_add (fun term => monomial term x y z) remaining
          (0 + monomial term x y z)]
      simp only [monomial]
      ring

theorem rawScale_eval (scalar : Rat) (poly : List RegionTerm) (x y z : Rat) :
    eval (rawScale scalar poly) x y z = scalar * eval poly x y z := by
  unfold rawScale eval
  rw [List.foldl_map]
  rw [rawScale_foldl scalar poly 0 x y z, zero_add]

theorem scale_eval (scalar : Rat) (poly : List RegionTerm) (x y z : Rat) :
    eval (scale scalar poly) x y z = scalar * eval poly x y z := by
  unfold scale
  rw [normalize_eval, rawScale_eval]

theorem neg_eval (poly : List RegionTerm) (x y z : Rat) :
    eval (neg poly) x y z = -eval poly x y z := by
  unfold neg
  rw [rawScale_eval]
  ring

theorem sub_eval (left right : List RegionTerm) (x y z : Rat) :
    eval (sub left right) x y z =
      eval left x y z - eval right x y z := by
  unfold sub
  rw [normalize_eval, eval_append, neg_eval]
  ring

theorem mulTerm_eval (left right : RegionTerm) (x y z : Rat) :
    monomial (mulTerm left right) x y z =
      monomial left x y z * monomial right x y z := by
  simp only [monomial, mulTerm]
  rw [pow_add, pow_add, pow_add]
  ring

private theorem mulTerms_eval (term : RegionTerm) (right : List RegionTerm) (x y z : Rat) :
    eval (right.map (mulTerm term)) x y z =
      monomial term x y z * eval right x y z := by
  induction right with
  | nil => simp [eval]
  | cons r remaining ih =>
      simp only [List.map_cons, eval, List.foldl_cons]
      simp only [eval_foldl]
      rw [mulTerm_eval, ih]
      ring

private theorem mulRaw_eval (left right : List RegionTerm) (x y z : Rat) :
    ∀ acc, eval (left.foldl (fun acc' lt => acc' ++ right.map (mulTerm lt)) acc) x y z =
      eval acc x y z + eval left x y z * eval right x y z := by
  induction left with
  | nil => intro acc; simp [eval]
  | cons lt remaining ih =>
      intro acc
      simp only [List.foldl_cons]
      rw [ih]
      simp only [eval, List.foldl_cons]
      simp only [eval_foldl]
      rw [eval_append, mulTerms_eval]
      ring

theorem mul_eval (left right : List RegionTerm) (x y z : Rat) :
    eval (mul left right) x y z =
      eval left x y z * eval right x y z := by
  unfold mul
  rw [normalize_eval, mulRaw_eval]
  simp [eval]

theorem pow_eval (poly : List RegionTerm) (exponent : Nat) (x y z : Rat) :
    eval (pow poly exponent) x y z = eval poly x y z ^ exponent := by
  induction exponent with
  | zero => simp [pow, eval, List.foldl_cons, monomial]
  | succ e ih => simp [pow, mul_eval, ih, pow_succ]

theorem variable_eval (axis : Nat) (x y z : Rat) :
    eval (variablePolynomial axis) x y z =
      (match axis with | 0 => x | 1 => y | _ => z) := by
  cases axis with
  | zero => simp [variablePolynomial, eval, List.foldl_cons, monomial]
  | succ axis =>
      cases axis with
      | zero => simp [variablePolynomial, eval, List.foldl_cons, monomial]
      | succ _ => simp [variablePolynomial, eval, List.foldl_cons, monomial]

theorem constant_eval (value : Rat) (x y z : Rat) :
    eval (constant value) x y z = value := by
  unfold constant
  split
  · rename_i hzero
    have hv : value = 0 := beq_iff_eq.mp hzero
    simp [eval, hv]
  · simp [eval, List.foldl_cons, monomial]

theorem map_sum_mul_left (f : RegionTerm → Rat) (r : Rat)
    (l : List RegionTerm) :
    r * (l.map f).sum = (l.map fun a => r * f a).sum := by
  induction l with
  | nil => simp
  | cons a rest ih => simp [List.map_cons, List.sum_cons, ih, mul_add]

theorem eval_sum (terms : List RegionTerm) (x y z : Rat) :
    eval terms x y z = (terms.map fun t => monomial t x y z).sum := by
  suffices h : ∀ acc, terms.foldl
      (fun total term => total + monomial term x y z) acc =
      acc + (terms.map fun t => monomial t x y z).sum by
    unfold eval
    rw [h 0, zero_add]
  induction terms with
  | nil => intro acc; simp
  | cons t rest ih =>
      intro acc
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih]
      ring

private def termKey (term : RegionTerm) : Nat × Nat × Nat :=
  (term.exponentX, term.exponentY, term.exponentZ)

private def keyBefore (a b : RegionTerm) : Prop :=
  a.exponentX < b.exponentX ∨
    (a.exponentX = b.exponentX ∧
      (a.exponentY < b.exponentY ∨
        (a.exponentY = b.exponentY ∧ a.exponentZ < b.exponentZ)))

private def keyLe (a b : RegionTerm) : Prop :=
  termKey a = termKey b ∨ keyBefore a b

private theorem precedes_iff (a b : RegionTerm) :
    precedes a b = true ↔ keyBefore a b := by
  simp [precedes, keyBefore]

private theorem sameKey_iff (a b : RegionTerm) :
    (a.exponentX == b.exponentX && a.exponentY == b.exponentY &&
      a.exponentZ == b.exponentZ) = true ↔ termKey a = termKey b := by
  simp [termKey, and_assoc]

private theorem keyLe_trans (a b c : RegionTerm) :
    keyLe a b → keyLe b c → keyLe a c := by
  simp only [keyLe, keyBefore, termKey, Prod.mk.injEq]
  omega

private theorem keyLe_of_not_before (a b : RegionTerm) :
    ¬keyBefore a b → keyLe b a := by
  simp only [keyLe, keyBefore, termKey, Prod.mk.injEq]
  omega

private theorem keyLe_antisymm (a b : RegionTerm) :
    keyLe a b → keyLe b a → termKey a = termKey b := by
  simp only [keyLe, keyBefore, termKey, Prod.mk.injEq]
  omega

private theorem keyLe_replace_right (a b c : RegionTerm)
    (h : termKey b = termKey c) : keyLe a b → keyLe a c := by
  simp only [keyLe, keyBefore, termKey, Prod.mk.injEq] at *
  omega

private def wellFormed (terms : List RegionTerm) : Prop :=
  terms.Pairwise (fun a b => termKey a ≠ termKey b) ∧
    ∀ term ∈ terms, term.coefficient ≠ 0

private theorem mergeTerm_keys (term : RegionTerm) (terms : List RegionTerm) :
    ∀ u ∈ mergeTerm term terms,
      termKey u = termKey term ∨ ∃ v ∈ terms, termKey u = termKey v := by
  induction terms with
  | nil =>
      simp only [mergeTerm]
      split <;> simp_all [termKey]
  | cons current rest ih =>
      simp only [mergeTerm]
      split
      · rename_i hs
        have hk := (sameKey_iff current term).mp hs
        split <;> simp_all [termKey] <;> grind
      · simp only [List.mem_cons]
        intro u hu
        rcases hu with rfl | hu
        · exact Or.inr ⟨u, by simp, rfl⟩
        · rcases ih u hu with h | ⟨v, hv, h⟩
          · exact Or.inl h
          · exact Or.inr ⟨v, by simp [hv], h⟩

private theorem mergeTerm_wellFormed (term : RegionTerm) (terms : List RegionTerm)
    (h : wellFormed terms) : wellFormed (mergeTerm term terms) := by
  induction terms with
  | nil =>
      simp only [mergeTerm]
      split <;> simp_all [wellFormed]
  | cons current rest ih =>
      obtain ⟨hp, hn⟩ := h
      obtain ⟨hd, ht⟩ := List.pairwise_cons.mp hp
      have hr : wellFormed rest := ⟨ht, fun u hu => hn u (by simp [hu])⟩
      simp only [mergeTerm]
      split
      · split
        · exact hr
        · rename_i hz
          refine ⟨List.pairwise_cons.mpr ⟨?_, ht⟩, ?_⟩
          · simpa [termKey] using hd
          · simpa only [List.mem_cons, forall_eq_or_imp, ne_eq, beq_iff_eq] using
              And.intro (by simpa using hz) hr.2
      · rename_i hs
        have hk : termKey current ≠ termKey term :=
          fun he => hs ((sameKey_iff current term).mpr he)
        have hm := ih hr
        refine ⟨List.pairwise_cons.mpr ⟨?_, hm.1⟩, ?_⟩
        · intro u hu
          rcases mergeTerm_keys term rest u hu with he | ⟨v, hv, he⟩
          · exact fun hc => hk (hc.trans he)
          · exact fun hc => hd v hv (hc.trans he)
        · simp only [List.mem_cons, forall_eq_or_imp]
          exact ⟨hn current (by simp), hm.2⟩

private theorem mergeAll_wellFormed (terms acc : List RegionTerm)
    (h : wellFormed acc) :
    wellFormed (terms.foldl (fun a t => mergeTerm t a) acc) := by
  induction terms generalizing acc with
  | nil => exact h
  | cons t rest ih => exact ih _ (mergeTerm_wellFormed t acc h)

private theorem insertSorted_perm (term : RegionTerm) (terms : List RegionTerm) :
    (insertSorted term terms).Perm (term :: terms) := by
  induction terms with
  | nil => exact List.Perm.refl _
  | cons current rest ih =>
      simp only [insertSorted]
      split
      · exact List.Perm.refl _
      · exact (ih.cons current).trans (List.Perm.swap _ _ _)

private theorem sortAll_perm (terms acc : List RegionTerm) :
    (terms.foldl (fun a t => insertSorted t a) acc).Perm (terms ++ acc) := by
  induction terms generalizing acc with
  | nil => simp
  | cons t rest ih =>
      simp only [List.foldl_cons]
      exact (ih _).trans
        ((List.Perm.append_left rest (insertSorted_perm t acc)).trans
          (by simpa using List.perm_middle))

private theorem insertSorted_ordered (term : RegionTerm) (terms : List RegionTerm)
    (h : terms.Pairwise keyLe) : (insertSorted term terms).Pairwise keyLe := by
  induction terms with
  | nil => simp [insertSorted]
  | cons current rest ih =>
      obtain ⟨hh, ht⟩ := List.pairwise_cons.mp h
      simp only [insertSorted]
      split
      · rename_i hb
        have hc : keyLe term current := Or.inr ((precedes_iff term current).mp hb)
        apply List.pairwise_cons.mpr
        refine ⟨?_, h⟩
        intro u hu
        rcases List.mem_cons.mp hu with rfl | hu
        · exact hc
        · exact keyLe_trans term current u hc (hh u hu)
      · rename_i hb
        have hc : keyLe current term := keyLe_of_not_before term current
          (fun hp => hb ((precedes_iff term current).mpr hp))
        apply List.pairwise_cons.mpr
        refine ⟨?_, ih ht⟩
        intro u hu
        have hm := (insertSorted_perm term rest).mem_iff.mp hu
        rcases List.mem_cons.mp hm with rfl | hu
        · exact hc
        · exact hh u hu

private theorem sortAll_ordered (terms acc : List RegionTerm)
    (h : acc.Pairwise keyLe) :
    (terms.foldl (fun a t => insertSorted t a) acc).Pairwise keyLe := by
  induction terms generalizing acc with
  | nil => exact h
  | cons t rest ih => exact ih _ (insertSorted_ordered t acc h)

private theorem normalize_wellFormed (terms : List RegionTerm) :
    wellFormed (normalize terms) := by
  let merged := terms.foldl (fun a t => mergeTerm t a) []
  have hm : wellFormed merged := mergeAll_wellFormed terms [] (by simp [wellFormed])
  have hp : (normalize terms).Perm merged := by
    simpa [normalize, merged] using sortAll_perm merged []
  refine ⟨(hp.pairwise_iff (fun h => Ne.symm h)).mpr hm.1, ?_⟩
  exact fun t ht => hm.2 t (hp.mem_iff.mp ht)

private theorem normalize_ordered (terms : List RegionTerm) :
    (normalize terms).Pairwise keyLe :=
  sortAll_ordered _ [] (by simp)

private noncomputable def termRepresentation (term : RegionTerm) :
    AddMonoidAlgebra Rat (Nat × Nat × Nat) :=
  AddMonoidAlgebra.single (termKey term) term.coefficient

private noncomputable def representation (terms : List RegionTerm) :
    AddMonoidAlgebra Rat (Nat × Nat × Nat) :=
  (terms.map termRepresentation).sum

private theorem representation_cons (t : RegionTerm) (ts : List RegionTerm) :
    representation (t :: ts) = termRepresentation t + representation ts := by
  simp [representation]

private theorem representation_append (p q : List RegionTerm) :
    representation (p ++ q) = representation p + representation q := by
  simp [representation]

private theorem mergeTerm_representation (term : RegionTerm) (terms : List RegionTerm) :
    representation (mergeTerm term terms) =
      termRepresentation term + representation terms := by
  induction terms with
  | nil =>
      simp only [mergeTerm]
      split
      · rename_i hz
        simp [representation, termRepresentation, beq_iff_eq.mp hz]
      · simp [representation]
  | cons current rest ih =>
      simp only [mergeTerm]
      split
      · rename_i hs
        have hk := (sameKey_iff current term).mp hs
        split
        · rename_i hz
          have hcoef : current.coefficient + term.coefficient = 0 :=
            beq_iff_eq.mp hz
          have hm : termRepresentation current + termRepresentation term = 0 := by
            unfold termRepresentation
            rw [← hk, ← AddMonoidAlgebra.single_add, hcoef,
              AddMonoidAlgebra.single_zero]
          have hm' : termRepresentation term + termRepresentation current = 0 :=
            (add_comm _ _).trans hm
          rw [representation_cons, ← add_assoc, hm', zero_add]
        · simp only [representation_cons, termRepresentation]
          have he : termKey
              ⟨current.coefficient + term.coefficient, current.exponentX,
                current.exponentY, current.exponentZ⟩ = termKey current := rfl
          rw [he, ← hk, AddMonoidAlgebra.single_add]
          abel
      · rw [representation_cons, ih, representation_cons]
        abel

private theorem mergeAll_representation (terms acc : List RegionTerm) :
    representation (terms.foldl (fun a t => mergeTerm t a) acc) =
      representation acc + representation terms := by
  induction terms generalizing acc with
  | nil => simp [representation]
  | cons t rest ih =>
      simp only [List.foldl_cons]
      rw [ih, mergeTerm_representation, representation_cons]
      abel

private theorem normalize_representation (terms : List RegionTerm) :
    representation (normalize terms) = representation terms := by
  unfold normalize
  rw [representation, (sortAll_perm _ []).map termRepresentation |>.sum_eq]
  simp only [List.append_nil]
  change representation (terms.foldl (fun a t => mergeTerm t a) []) =
    representation terms
  rw [mergeAll_representation]
  simp [representation]

private theorem representation_coeff_zero (terms : List RegionTerm) (k : Nat × Nat × Nat)
    (h : ∀ t ∈ terms, termKey t ≠ k) : (representation terms).coeff k = 0 := by
  induction terms with
  | nil => simp [representation]
  | cons t rest ih =>
      rw [representation_cons]
      simp [termRepresentation, Finsupp.single_apply, h t (by simp),
        ih (fun u hu => h u (by simp [hu]))]

private theorem representation_head_coeff (t : RegionTerm) (ts : List RegionTerm)
    (h : wellFormed (t :: ts)) :
    (representation (t :: ts)).coeff (termKey t) = t.coefficient := by
  obtain ⟨hp, _⟩ := h
  have hd := (List.pairwise_cons.mp hp).1
  rw [representation_cons]
  simp [termRepresentation, representation_coeff_zero ts (termKey t)
    (fun u hu => Ne.symm (hd u hu))]

private theorem representation_key_exists (terms : List RegionTerm) (k : Nat × Nat × Nat)
    (h : (representation terms).coeff k ≠ 0) :
    ∃ t ∈ terms, termKey t = k := by
  by_contra hn
  apply h
  exact representation_coeff_zero terms k (by simpa using hn)

private theorem canonical_representation_injective (p q : List RegionTerm)
    (hp : wellFormed p) (hq : wellFormed q)
    (sp : p.Pairwise keyLe) (sq : q.Pairwise keyLe)
    (he : representation p = representation q) : p = q := by
  induction p generalizing q with
  | nil =>
      cases q with
      | nil => rfl
      | cons b bs =>
          have hc := congrArg (fun r => r.coeff (termKey b)) he
          rw [representation_head_coeff b bs hq] at hc
          have hz : b.coefficient = 0 := by simpa [representation] using hc.symm
          exact False.elim (hq.2 b (by simp) hz)
  | cons a as ih =>
      cases q with
      | nil =>
          have hc := congrArg (fun r => r.coeff (termKey a)) he
          rw [representation_head_coeff a as hp] at hc
          have hz : a.coefficient = 0 := by simpa [representation] using hc
          exact False.elim (hp.2 a (by simp) hz)
      | cons b bs =>
          have ha : (representation (b :: bs)).coeff (termKey a) ≠ 0 := by
            rw [← he, representation_head_coeff a as hp]
            exact hp.2 a (by simp)
          have hb : (representation (a :: as)).coeff (termKey b) ≠ 0 := by
            rw [he, representation_head_coeff b bs hq]
            exact hq.2 b (by simp)
          obtain ⟨u, hu, huKey⟩ := representation_key_exists _ _ ha
          obtain ⟨v, hv, hvKey⟩ := representation_key_exists _ _ hb
          have hba : keyLe b a := by
            rcases List.mem_cons.mp hu with rfl | hu
            · exact Or.inl huKey
            · exact keyLe_replace_right b u a huKey
                ((List.pairwise_cons.mp sq).1 u hu)
          have hab : keyLe a b := by
            rcases List.mem_cons.mp hv with rfl | hv
            · exact Or.inl hvKey
            · exact keyLe_replace_right a v b hvKey
                ((List.pairwise_cons.mp sp).1 v hv)
          have hk := keyLe_antisymm a b hab hba
          have hc := congrArg (fun r => r.coeff (termKey a)) he
          rw [representation_head_coeff a as hp, hk,
            representation_head_coeff b bs hq] at hc
          have ht : a = b := by
            cases a; cases b
            simp_all [termKey, Prod.mk.injEq]
          subst b
          have hr : representation as = representation bs := by
            simpa only [representation_cons, add_left_cancel_iff] using he
          have hp' : wellFormed as :=
            ⟨(List.pairwise_cons.mp hp.1).2, fun u hu => hp.2 u (by simp [hu])⟩
          have hq' : wellFormed bs :=
            ⟨(List.pairwise_cons.mp hq.1).2, fun u hu => hq.2 u (by simp [hu])⟩
          exact congrArg (List.cons a)
            (ih bs hp' hq' (List.pairwise_cons.mp sp).2
              (List.pairwise_cons.mp sq).2 hr)

private theorem normalize_eq_of_representation (p q : List RegionTerm)
    (h : representation p = representation q) : normalize p = normalize q := by
  apply canonical_representation_injective _ _ (normalize_wellFormed p)
    (normalize_wellFormed q) (normalize_ordered p) (normalize_ordered q)
  simpa only [normalize_representation] using h

private theorem rawScale_representation (s : Rat) (p : List RegionTerm) :
    representation (rawScale s p) =
      AddMonoidAlgebra.single (0 : Nat × Nat × Nat) s * representation p := by
  induction p with
  | nil => simp [rawScale, representation]
  | cons t ts ih =>
      simp only [rawScale, List.map_cons, representation_cons, mul_add]
      rw [← ih]
      simp [rawScale, termRepresentation, termKey, AddMonoidAlgebra.single_mul_single]

private theorem mulTerm_representation (a b : RegionTerm) :
    termRepresentation (mulTerm a b) = termRepresentation a * termRepresentation b := by
  simp [termRepresentation, termKey, mulTerm, AddMonoidAlgebra.single_mul_single]

private theorem mulMap_representation (t : RegionTerm) (p : List RegionTerm) :
    representation (p.map (mulTerm t)) = termRepresentation t * representation p := by
  induction p with
  | nil => simp [representation]
  | cons u us ih =>
      simp only [List.map_cons, representation_cons, mulTerm_representation, ih, mul_add]

private theorem mul_representation (p q : List RegionTerm) :
    representation (mul p q) = representation p * representation q := by
  rw [mul_eq_foldr, normalize_representation]
  induction p with
  | nil => simp [representation]
  | cons t ts ih =>
      simp only [List.foldr_cons, representation_append, mulMap_representation,
        representation_cons, add_mul]
      rw [ih]

theorem normalize_idempotent (p : List RegionTerm) :
    normalize (normalize p) = normalize p :=
  normalize_eq_of_representation _ _ (normalize_representation p)

theorem add_normalize_left (p q : List RegionTerm) :
    add (normalize p) q = add p q := by
  apply normalize_eq_of_representation
  simp only [representation_append, normalize_representation]

theorem add_normalize_right (p q : List RegionTerm) :
    add p (normalize q) = add p q := by
  apply normalize_eq_of_representation
  simp only [representation_append, normalize_representation]

theorem scale_normalize (s : Rat) (p : List RegionTerm) :
    scale s (normalize p) = scale s p := by
  apply normalize_eq_of_representation
  simp only [rawScale_representation, normalize_representation]

theorem mul_normalize_left (p q : List RegionTerm) :
    mul (normalize p) q = mul p q := by
  have he := normalize_eq_of_representation (mul (normalize p) q) (mul p q)
    (by simp only [mul_representation, normalize_representation])
  simpa only [mul, normalize_idempotent] using he

theorem mul_normalize_right (p q : List RegionTerm) :
    mul p (normalize q) = mul p q := by
  have he := normalize_eq_of_representation (mul p (normalize q)) (mul p q)
    (by simp only [mul_representation, normalize_representation])
  simpa only [mul, normalize_idempotent] using he

theorem mul_one_left (p : List RegionTerm) :
    mul [⟨1, 0, 0, 0⟩] p = normalize p := by
  rw [mul_eq_foldr]
  congr 1
  simp only [List.foldr_cons, List.foldr_nil, List.append_nil]
  have hm : mulTerm ⟨1, 0, 0, 0⟩ = id := by
    funext t
    cases t
    simp [mulTerm]
  rw [hm, List.map_id]

end RegionPolynomial

end NecoGeometryVerify
