import NecoGeometryVerify.Checker
import NecoCalculusVerify.Soundness

set_option autoImplicit false

namespace NecoGeometryVerify

open NecoAlgebraVerify
open NecoCalculusVerify

theorem membershipClassification_zero_is_boundary :
    membershipClassification 0 = "boundary" := rfl

theorem surfaceExactValue_matches_calculus
    (x y z denominator : List SurfaceTerm) (u v : Rat) :
    match surfaceExactValue x y z denominator u v with
    | some (observedX, observedY, observedZ) =>
        surfaceExact x y z denominator u v observedX observedY observedZ
    | none => surfaceValue denominator u v = 0 := by
  unfold surfaceExactValue surfaceExact
  split <;> simp_all

theorem surfaceExactValue_of_nonzero
    (x y z denominator : List SurfaceTerm) (u v : Rat)
    (nonzero : surfaceValue denominator u v ≠ 0) :
    surfaceExactValue x y z denominator u v = some
      (surfaceValue x u v / surfaceValue denominator u v,
       surfaceValue y u v / surfaceValue denominator u v,
       surfaceValue z u v / surfaceValue denominator u v) := by
  unfold surfaceExactValue
  dsimp only
  split
  · rename_i zero
    exact False.elim (nonzero (beq_iff_eq.mp zero))
  · rfl

theorem surfaceCheckedRange_contains
    (x y z denominator : List SurfaceTerm) (u v : NecoAlgebraVerify.Interval)
    (output : CoordinateIntervals)
    (checked : surfaceCheckedRange x y z denominator u v = some output) :
    ∀ (pointU pointV : Rat), u.Contains pointU → v.Contains pointV →
      surfaceValue denominator pointU pointV ≠ 0 ∧
      output.x.Contains (surfaceValue x pointU pointV /
        surfaceValue denominator pointU pointV) ∧
      output.y.Contains (surfaceValue y pointU pointV /
        surfaceValue denominator pointU pointV) ∧
      output.z.Contains (surfaceValue z pointU pointV /
        surfaceValue denominator pointU pointV) := by
  unfold surfaceCheckedRange at checked
  dsimp only at checked
  split at checked
  · rename_i excludesZero
    cases checked
    intro pointU pointV uContained vContained
    have denominatorContained := surfaceRange_contains denominator u v
      pointU pointV uContained vContained
    have denominatorValid := surfaceRange_valid denominator u v
    have denominatorNonzero : surfaceValue denominator pointU pointV ≠ 0 :=
      NecoAlgebraVerify.excludesZero_sound excludesZero denominatorContained
    have xContained := NecoAlgebraVerify.Interval.div_contains
      (surfaceRange_valid x u v) denominatorValid excludesZero
      (surfaceRange_contains x u v pointU pointV uContained vContained)
      denominatorContained
    have yContained := NecoAlgebraVerify.Interval.div_contains
      (surfaceRange_valid y u v) denominatorValid excludesZero
      (surfaceRange_contains y u v pointU pointV uContained vContained)
      denominatorContained
    have zContained := NecoAlgebraVerify.Interval.div_contains
      (surfaceRange_valid z u v) denominatorValid excludesZero
      (surfaceRange_contains z u v pointU pointV uContained vContained)
      denominatorContained
    exact ⟨denominatorNonzero, xContained, yContained, zContained⟩
  · simp at checked

end NecoGeometryVerify
