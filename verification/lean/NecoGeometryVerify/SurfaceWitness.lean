import NecoGeometryVerify.Region
import NecoCalculusVerify.Semantics

set_option autoImplicit false

namespace NecoGeometryVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

structure ChartIdentity where
  domainId : String
  version : Nat
  deriving Repr, BEq, DecidableEq

def decodeChartIdentity (value : Value) : Option ChartIdentity := do
  let fields ← jsonObjectWithExactFields? value ["domainId", "version"]
  let domainId ← jsonStringField? fields "domainId"
  let version ← decodeDecimalNatField fields "version"
  if domainId.isEmpty then none else some { domainId, version }

def decodeSurfaceTermsGeometry (value : Value) : Option (List SurfaceTerm) := do
  let values ← match value with
    | .array values => some values
    | _ => none
  if values.length > 4096 then none
  let decoded ← values.mapM fun term => do
    let fields ← jsonObjectWithExactFields? term ["coefficient", "exponentU", "exponentV"]
    let coefficient ← decodeNormalizedRational (← jsonField? fields "coefficient")
    let exponentU ← decodeDecimalNatField fields "exponentU"
    let exponentV ← decodeDecimalNatField fields "exponentV"
    if exponentU ≤ 256 && exponentV ≤ 256 then
      some ({ coefficient, exponentU, exponentV } : SurfaceTerm)
    else
      none
  some decoded

def decodeExactInteger (value : Value) : Option Int :=
  jsonString? value >>= decodeDecimalInteger

def parameterInDomain (value : Int) : Bool :=
  0 ≤ value && value ≤ 8

def enclosureInDomain (interval : NecoCalculusVerify.Interval) : Bool :=
  0 ≤ interval.lower && interval.upper ≤ 8 && decide interval.Valid

private def decodeDecimalNatArray (value : Value) : Option (List Nat) :=
  match value with
  | .array values => values.mapM (fun item => jsonString? item >>= decodeDecimalNat)
  | _ => none

private def decodeDecisionGeometry (value : Value) : Option (String × List Nat) := do
  let kindFields ← jsonFields? value
  let kind ← jsonStringField? kindFields "kind"
  if kind == "equality" then
    let fields ← jsonObjectWithExactFields? value ["expressions", "kind"]
    let expressions ← decodeDecimalNatArray (← jsonField? fields "expressions")
    pure (kind, expressions)
  else
    let fields ← jsonObjectWithExactFields? value ["expression", "kind"]
    let expression ← decodeDecimalNat (← jsonStringField? fields "expression")
    pure (kind, [expression])

def decodeProvenanceGeometry (value : Value) : Option Provenance := do
  let fields ← jsonObjectWithExactFields? value
    ["consumerId", "decision", "expressions", "owner", "precision", "source"]
  let consumerId ← decodeDecimalNatField fields "consumerId"
  let expressions ← decodeDecimalNatArray (← jsonField? fields "expressions")
  if expressions.isEmpty then none else pure ()
  let (decisionKind, decisionExpressions) ←
    decodeDecisionGeometry (← jsonField? fields "decision")
  let owner ← decodeDecimalNatField fields "owner"
  let precision ← decodeDecimalNatField fields "precision"
  let sourceFields ← jsonObjectWithExactFields? (← jsonField? fields "source")
    ["publicApiIdentity", "revisionIdentity"]
  let sourcePublicApiIdentity ← jsonStringField? sourceFields "publicApiIdentity"
  let sourceRevisionIdentity ← jsonStringField? sourceFields "revisionIdentity"
  pure {
    consumerId
    expressions
    decisionKind
    decisionExpressions
    owner
    precision
    sourcePublicApiIdentity
    sourceRevisionIdentity
  }

def decodeExactPoint (value : Value) : Option (Rat × Rat × Rat) := do
  let fields ← jsonObjectWithExactFields? value ["x", "y", "z"]
  let x ← decodeNormalizedRational (← jsonField? fields "x")
  let y ← decodeNormalizedRational (← jsonField? fields "y")
  let z ← decodeNormalizedRational (← jsonField? fields "z")
  some (x, y, z)

def surfaceExactValue (x y z denominator : List SurfaceTerm) (u v : Rat) : Option (Rat × Rat × Rat) :=
  let divisor := surfaceValue denominator u v
  if divisor == 0 then none
  else some (
    surfaceValue x u v / divisor,
    surfaceValue y u v / divisor,
    surfaceValue z u v / divisor)

def surfaceCheckedRange (x y z denominator : List SurfaceTerm) (u v : NecoCalculusVerify.Interval) :
    Option CoordinateIntervals :=
  let denominatorRange := surfaceRange denominator u v
  if denominatorRange.ExcludesZero then some {
    x := NecoAlgebraVerify.Interval.div (surfaceRange x u v) denominatorRange
    y := NecoAlgebraVerify.Interval.div (surfaceRange y u v) denominatorRange
    z := NecoAlgebraVerify.Interval.div (surfaceRange z u v) denominatorRange
  } else none

end NecoGeometryVerify
