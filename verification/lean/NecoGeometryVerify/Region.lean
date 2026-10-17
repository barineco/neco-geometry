import NecoAlgebraVerify.Canonical
import NecoCalculusVerify.Protocol
import NecoCalculusVerify.Semantics
import NecoGeometryVerify.RegionPolynomial

set_option autoImplicit false

namespace NecoGeometryVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

def decodeDecimalNat (text : String) : Option Nat :=
  decodeUnsignedNat text

def decodeDecimalNatField (fields : JsonFields) (name : String) : Option Nat := do
  decodeDecimalNat (← jsonStringField? fields name)

def decodeRegionTerm (value : Value) : Option RegionTerm := do
  let fields ← jsonObjectWithExactFields? value ["coefficient", "exponentX", "exponentY", "exponentZ"]
  let coefficient ← decodeNormalizedRational (← jsonField? fields "coefficient")
  let exponentX ← decodeDecimalNatField fields "exponentX"
  let exponentY ← decodeDecimalNatField fields "exponentY"
  let exponentZ ← decodeDecimalNatField fields "exponentZ"
  if exponentX ≤ 256 && exponentY ≤ 256 && exponentZ ≤ 256 then
    some { coefficient, exponentX, exponentY, exponentZ }
  else
    none

def decodeRegionTerms (value : Value) : Option (List RegionTerm) :=
  match value with
  | .array values =>
      if values.length ≤ 4096 then values.mapM decodeRegionTerm else none
  | _ => none

def maxRegionPolynomialTerms : Nat := 256

def regionPolynomialTermLimit (terms : List RegionTerm) : Bool :=
  maxRegionPolynomialTerms < (RegionPolynomial.normalize terms).length

def isAffineCoordinate (terms : List RegionTerm) : Bool :=
  (RegionPolynomial.normalize terms).all fun term =>
    term.exponentX + term.exponentY + term.exponentZ ≤ 1

def isAffineMap (x y z : List RegionTerm) : Bool :=
  isAffineCoordinate x && isAffineCoordinate y && isAffineCoordinate z

def substitutePolynomial (source x y z : List RegionTerm) : List RegionTerm :=
  (source.foldl (fun acc term =>
    RegionPolynomial.add acc
      (RegionPolynomial.scale term.coefficient
        (RegionPolynomial.mul (RegionPolynomial.mul (RegionPolynomial.pow x term.exponentX)
          (RegionPolynomial.pow y term.exponentY))
          (RegionPolynomial.pow z term.exponentZ)))) [])

inductive RegionFormula where
  | atom (atomId : String)
  | not (operand : RegionFormula)
  | and (operands : List RegionFormula)
  | or (operands : List RegionFormula)
  | true
  | false
  deriving Repr, BEq

private def formulaRank : RegionFormula → Nat
  | .atom _ => 0
  | .not _ => 1
  | .and _ => 2
  | .or _ => 3
  | .true => 4
  | .false => 5

mutual
partial def compareFormula (left right : RegionFormula) : Ordering :=
  match compare (formulaRank left) (formulaRank right) with
  | .lt => .lt
  | .gt => .gt
  | .eq =>
      match left, right with
      | .atom a, .atom b => compare a b
      | .not a, .not b => compareFormula a b
      | .and a, .and b | .or a, .or b => compareFormulaList a b
      | _, _ => .eq

partial def compareFormulaList (left right : List RegionFormula) : Ordering :=
  match left, right with
  | [], [] => .eq
  | [], _ => .lt
  | _, [] => .gt
  | a :: as, b :: bs =>
      match compareFormula a b with
      | .eq => compareFormulaList as bs
      | ordering => ordering
end

private def insertFormula (formula : RegionFormula) : List RegionFormula → List RegionFormula
  | [] => [formula]
  | current :: remaining =>
      match compareFormula formula current with
      | .lt => formula :: current :: remaining
      | .eq => current :: remaining
      | .gt => current :: insertFormula formula remaining

mutual
partial def normalizeFormula : RegionFormula → Option RegionFormula
  | .atom atomId => if atomId.isEmpty then none else some (.atom atomId)
  | .true => some .true
  | .false => some .false
  | .not inner =>
      match normalizeFormula inner with
      | none => none
      | some .true => some .false
      | some .false => some .true
      | some (.not inner) => some inner
      | some normalized => some (.not normalized)
  | .and operands => normalizeOperands true operands
  | .or operands => normalizeOperands false operands

partial def normalizeOperands (isAnd : Bool) (operands : List RegionFormula) : Option RegionFormula :=
  if operands.isEmpty then none
  else
    let rec collect (remaining : List RegionFormula) (acc : List RegionFormula) : Option RegionFormula :=
      match remaining with
      | [] =>
          let sorted := acc.foldl (fun items formula => insertFormula formula items) []
          match sorted with
          | [] => some (if isAnd then .true else .false)
          | [one] => some one
          | many => some (if isAnd then .and many else .or many)
      | operand :: rest =>
          match normalizeFormula operand with
          | none => none
          | some .false => if isAnd then some .false else collect rest acc
          | some .true => if isAnd then collect rest acc else some .true
          | some normalized => collect rest (acc ++ [normalized])
    collect operands []
end

partial def decodeFormula (value : Value) : Option RegionFormula :=
  match jsonObjectWithExactFields? value ["kind"],
      jsonObjectWithExactFields? value ["atomId", "kind"],
      jsonObjectWithExactFields? value ["kind", "operand"],
      jsonObjectWithExactFields? value ["kind", "operands"] with
  | some fields, _, _, _ =>
      match jsonStringField? fields "kind" with
      | some "true" => some .true
      | some "false" => some .false
      | _ => none
  | _, some fields, _, _ =>
      if jsonStringField? fields "kind" == some "atom" then
        (jsonStringField? fields "atomId").map RegionFormula.atom
      else none
  | _, _, some fields, _ =>
      if jsonStringField? fields "kind" == some "not" then
        (jsonField? fields "operand" >>= decodeFormula).map RegionFormula.not
      else none
  | _, _, _, some fields =>
      match jsonStringField? fields "kind", jsonField? fields "operands" with
      | some "and", some (.array operands) =>
          operands.mapM decodeFormula |>.map RegionFormula.and
      | some "or", some (.array operands) =>
          operands.mapM decodeFormula |>.map RegionFormula.or
      | _, _ => none
  | _, _, _, _ => none

def decodeDecimalIntField (fields : JsonFields) (name : String) : Option Int := do
  decodeDecimalInteger (← jsonStringField? fields name)

def membershipSign (x : Int) : String :=
  if x < 0 then "negative" else if x == 0 then "zero" else "positive"

def membershipClassification (x : Int) : String :=
  if x == 0 then "boundary" else if 0 < x then "inside" else "outside"

inductive GeometryFailureKind where
  | parameterOutsideDomain
  | invalidRegionAffineMap
  | regionPolynomialTermLimit (requestedTerms maximumTerms : Nat)
  | malformedRegionFormula
  | regionPointDimensionMismatch
  deriving Repr, BEq, DecidableEq

def decodeGeometryFailure (value : Value) : Option GeometryFailureKind :=
  match jsonObjectWithExactFields? value ["variant"],
      jsonObjectWithExactFields? value ["maximumTerms", "requestedTerms", "variant"] with
  | some fields, _ =>
      match jsonStringField? fields "variant" with
      | some "parameter-outside-domain" => some .parameterOutsideDomain
      | some "invalid-region-affine-map" => some .invalidRegionAffineMap
      | some "malformed-region-formula" => some .malformedRegionFormula
      | some "region-point-dimension-mismatch" => some .regionPointDimensionMismatch
      | _ => none
  | _, some fields =>
      if jsonStringField? fields "variant" == some "region-polynomial-term-limit" then
        match decodeDecimalNatField fields "requestedTerms",
            decodeDecimalNatField fields "maximumTerms" with
        | some requested, some maximum => some (.regionPolynomialTermLimit requested maximum)
        | _, _ => none
      else none
  | _, _ => none

end NecoGeometryVerify
