import NecoLean.Digest
import NecoCalculusVerify.Protocol
import NecoGeometryVerify.Protocol
import NecoGeometryVerify.Region
import NecoGeometryVerify.SurfaceWitness

set_option autoImplicit false

namespace NecoGeometryVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

private def decodeObservationPayload (value : Value) : Option (Bool × Value) :=
  match jsonObjectWithExactFields? value ["kind", "value"],
      jsonObjectWithExactFields? value ["failure", "kind"] with
  | some fields, _ => do
      if (← jsonStringField? fields "kind") != "success" then none
      some (true, ← jsonField? fields "value")
  | _, some fields => do
      if (← jsonStringField? fields "kind") != "failure" then none
      some (false, ← jsonField? fields "failure")
  | _, _ => none

private def termsEqual (left right : List RegionTerm) : Bool :=
  left == right

private def checkPolynomialNormalization (input observation : Value) : Option Verdict := do
  let fields ← jsonObjectWithExactFields? input ["rawTerms"]
  let raw ← decodeRegionTerms (← jsonField? fields "rawTerms")
  let normalized := RegionPolynomial.normalize raw
  let payload ← decodeObservationPayload observation
  match payload with
  | (true, value) =>
      let observedFields ← jsonObjectWithExactFields? value ["terms"]
      let observed ← decodeRegionTerms (← jsonField? observedFields "terms")
      if regionPolynomialTermLimit raw then some .refuted
      else if termsEqual observed normalized then some .verified else some .refuted
  | (false, failure) =>
      match decodeGeometryFailure failure with
      | some (.regionPolynomialTermLimit requested maximum) =>
          if regionPolynomialTermLimit raw &&
              requested == (RegionPolynomial.normalize raw).length &&
              maximum == maxRegionPolynomialTerms then
            some .verified
          else some .refuted
      | _ => some .refuted

private def checkAffineSubstitution (input observation : Value) : Option Verdict := do
  let fields ← jsonObjectWithExactFields? input ["map", "source"]
  let mapFields ← jsonObjectWithExactFields? (← jsonField? fields "map") ["x", "y", "z"]
  let sourceFields ← jsonObjectWithExactFields? (← jsonField? fields "source") ["terms"]
  let mapX ← decodeRegionTerms (← jsonField? mapFields "x")
  let mapY ← decodeRegionTerms (← jsonField? mapFields "y")
  let mapZ ← decodeRegionTerms (← jsonField? mapFields "z")
  let source ← decodeRegionTerms (← jsonField? sourceFields "terms")
  let payload ← decodeObservationPayload observation
  match payload with
  | (true, value) =>
      if !isAffineMap mapX mapY mapZ then some .refuted
      else
        let observedFields ← jsonObjectWithExactFields? value ["terms"]
        let observed ← decodeRegionTerms (← jsonField? observedFields "terms")
        if termsEqual observed (substitutePolynomial source mapX mapY mapZ) then some .verified
        else some .refuted
  | (false, failure) =>
      match decodeGeometryFailure failure with
      | some .invalidRegionAffineMap =>
          if isAffineMap mapX mapY mapZ then some .refuted else some .verified
      | _ => some .refuted

private def expectedFormula (variant : String) : Option RegionFormula :=
  match variant with
  | "true" => normalizeFormula .true
  | "and" => normalizeFormula (.and [.atom "a", .true])
  | "not" => normalizeFormula (.not (.not (.atom "a")))
  | "false" => normalizeFormula .false
  | "empty-and" => none
  | _ => none

private def checkFormulaConstruction (input observation : Value) : Option Verdict := do
  let fields ← jsonObjectWithExactFields? input ["variant"]
  let variant ← jsonStringField? fields "variant"
  let payload ← decodeObservationPayload observation
  match expectedFormula variant, payload with
  | some expected, (true, value) =>
      let observedFields ← jsonObjectWithExactFields? value ["formula"]
      let observed ← decodeFormula (← jsonField? observedFields "formula")
      if observed == expected then some .verified else some .refuted
  | none, (false, failure) =>
      match decodeGeometryFailure failure with
      | some .malformedRegionFormula => some .verified
      | _ => some .refuted
  | _, _ => some .refuted

private def checkMembership (input observation : Value) : Option Verdict := do
  let fields ← jsonObjectWithExactFields? input ["formulaVariant", "x", "y", "z"]
  let x ← decodeDecimalIntField fields "x"
  let payload ← decodeObservationPayload observation
  match payload with
  | (true, value) =>
      let observedFields ← jsonObjectWithExactFields? value ["atomSigns", "classification"]
      let classification ← jsonStringField? observedFields "classification"
      let atomSigns ← match jsonField? observedFields "atomSigns" with
        | some (.array signs) => some signs
        | _ => none
      let signFields ← match atomSigns with
        | [sign] => jsonObjectWithExactFields? sign ["atomId", "relation"]
        | _ => none
      let atomId ← jsonStringField? signFields "atomId"
      let relation ← jsonStringField? signFields "relation"
      if atomId == "a" && relation == membershipSign x &&
          classification == membershipClassification x then
        some .verified
      else some .refuted
  | (false, failure) =>
      match decodeGeometryFailure failure with
      | some .regionPointDimensionMismatch => some .verified
      | _ => some .refuted

private def checkSurfaceExact (input observation : Value) : Option Verdict := do
  let fields ← jsonObjectWithExactFields? input
    ["denominator", "identity", "u", "v", "x", "y", "z"]
  let identity ← decodeChartIdentity (← jsonField? fields "identity")
  let u ← decodeExactInteger (← jsonField? fields "u")
  let v ← decodeExactInteger (← jsonField? fields "v")
  let x ← decodeSurfaceTermsGeometry (← jsonField? fields "x")
  let y ← decodeSurfaceTermsGeometry (← jsonField? fields "y")
  let z ← decodeSurfaceTermsGeometry (← jsonField? fields "z")
  let denominator ← decodeSurfaceTermsGeometry (← jsonField? fields "denominator")
  let payload ← decodeObservationPayload observation
  match payload with
  | (true, value) =>
      if !parameterInDomain u || !parameterInDomain v then some .refuted
      else
        let observedFields ← jsonObjectWithExactFields? value ["identity", "point"]
        let observedIdentity ← decodeChartIdentity (← jsonField? observedFields "identity")
        let observedPoint ← decodeExactPoint (← jsonField? observedFields "point")
        match surfaceExactValue x y z denominator (Rat.ofInt u) (Rat.ofInt v) with
        | some (px, py, pz) =>
            if observedIdentity == identity && observedPoint == (px, py, pz) then some .verified
            else some .refuted
        | none => some .refuted
  | (false, failure) =>
      match decodeGeometryFailure failure with
      | some .parameterOutsideDomain =>
          if parameterInDomain u && parameterInDomain v then some .refuted else some .verified
      | _ => some .refuted

private def checkSurfaceChecked (input observation : Value) : Option Verdict := do
  let fields ← jsonObjectWithExactFields? input
    ["denominator", "identity", "provenance", "u", "v", "x", "y", "z"]
  let identity ← decodeChartIdentity (← jsonField? fields "identity")
  let u ← decodeDyadicInterval (← jsonField? fields "u")
  let v ← decodeDyadicInterval (← jsonField? fields "v")
  let x ← decodeSurfaceTermsGeometry (← jsonField? fields "x")
  let y ← decodeSurfaceTermsGeometry (← jsonField? fields "y")
  let z ← decodeSurfaceTermsGeometry (← jsonField? fields "z")
  let denominator ← decodeSurfaceTermsGeometry (← jsonField? fields "denominator")
  let inputProvenance ← decodeProvenanceGeometry (← jsonField? fields "provenance")
  let payload ← decodeObservationPayload observation
  match payload with
  | (true, value) =>
      if !enclosureInDomain u || !enclosureInDomain v then some .refuted
      else
        let observedFields ← jsonObjectWithExactFields? value
          ["identity", "provenance", "x", "y", "z"]
        let observedIdentity ← decodeChartIdentity (← jsonField? observedFields "identity")
        let observedProvenance ← decodeProvenanceGeometry (← jsonField? observedFields "provenance")
        let observedX ← decodeDyadicInterval (← jsonField? observedFields "x")
        let observedY ← decodeDyadicInterval (← jsonField? observedFields "y")
        let observedZ ← decodeDyadicInterval (← jsonField? observedFields "z")
        match surfaceCheckedRange x y z denominator u v with
        | some expected =>
            if observedIdentity == identity && observedProvenance == inputProvenance &&
                observedX.lower ≤ expected.x.lower && expected.x.upper ≤ observedX.upper &&
                observedY.lower ≤ expected.y.lower && expected.y.upper ≤ observedY.upper &&
                observedZ.lower ≤ expected.z.lower && expected.z.upper ≤ observedZ.upper then
              some .verified
            else some .refuted
        | none => some .refuted
  | (false, failure) =>
      match decodeGeometryFailure failure with
      | some .parameterOutsideDomain =>
          if enclosureInDomain u && enclosureInDomain v then some .refuted else some .verified
      | _ => some .refuted

def checkCase (value : Value) : CaseResult :=
  match jsonObjectWithExactFields? value ["caseId", "input", "observation", "operation"] with
  | none => { caseId := "", verdict := .invalid }
  | some fields =>
      match jsonStringField? fields "caseId", jsonStringField? fields "operation",
          jsonField? fields "input", jsonField? fields "observation" with
      | some caseId, some operation, some input, some observation =>
          if caseId.isEmpty then { caseId, verdict := .invalid }
          else
            let verdict :=
              match operation with
              | "region-polynomial-normalization" => checkPolynomialNormalization input observation
              | "region-affine-substitution" => checkAffineSubstitution input observation
              | "region-formula-construction" => checkFormulaConstruction input observation
              | "region-membership" => checkMembership input observation
              | "surface-witness-exact" => checkSurfaceExact input observation
              | "surface-witness-checked" => checkSurfaceChecked input observation
              | _ => none
            match verdict with
            | some verdict => { caseId, verdict }
            | none => { caseId, verdict := .invalid }
      | _, _, _, _ => { caseId := "", verdict := .invalid }

private def uniqueCaseIds : List CaseResult → Bool
  | [] => true
  | item :: remaining =>
      !item.caseId.isEmpty && !(remaining.any fun candidate => candidate.caseId == item.caseId) &&
        uniqueCaseIds remaining

private def checkedBatch (digest : String) (cases : List CaseResult) : CheckResult :=
  if uniqueCaseIds cases then .cases digest cases else .batchInvalid digest

def checkBytes (bytes : ByteArray) : CheckResult :=
  let digest := NecoLean.Digest.toHex (NecoLean.Digest.sha256 bytes)
  match decodeCanonicalJson bytes with
  | some (.object fields) =>
      match NecoLean.Json.encode (.object fields),
          jsonObjectWithExactFields? (.object fields) ["cases", "schema"],
          jsonField? fields "cases", jsonStringField? fields "schema" with
      | .ok encoded, some _, some (.array cases), some "neco.geometry.observation-batch.v1" =>
          if encoded == bytes then checkedBatch digest (cases.map checkCase)
          else .batchInvalid digest
      | _, _, _, _ => .batchInvalid digest
  | _ => .batchInvalid digest

private def verdictText : Verdict → String
  | .verified => "verified"
  | .refuted => "refuted"
  | .undetermined => "undetermined"
  | .invalid => "invalid"

def resultJson : CheckResult → Value
  | .batchInvalid digest =>
      .object [("batchVerdict", .string "invalid"), ("digest", .string digest)]
  | .cases digest items =>
      .object [
        ("cases", .array (items.map fun item =>
          .object [("caseId", .string item.caseId), ("verdict", .string (verdictText item.verdict))])),
        ("digest", .string digest)
      ]

def resultBytes (result : CheckResult) : ByteArray :=
  match NecoLean.Json.encode (resultJson result) with
  | .ok bytes => bytes
  | .error _ => "{\"batchVerdict\":\"invalid\",\"digest\":\"\"}".toUTF8

end NecoGeometryVerify
