import NecoGeometryVerify

open NecoGeometryVerify
open NecoCalculusVerify

private def verified : NecoCalculusVerify.CheckResult → Bool
  | .batchInvalid _ => false
  | .cases _ items => !items.isEmpty && items.all (fun item => item.verdict == .verified)

private def expectVerdict (name : String) (expected : NecoCalculusVerify.Verdict) :
    NecoCalculusVerify.CheckResult → IO Unit
  | .cases _ (item :: _) =>
      if item.verdict == expected then pure ()
      else throw (IO.userError s!"{name}: unexpected verdict")
  | _ => throw (IO.userError s!"{name}: expected a case verdict")

private def expectBatchInvalid (name : String) : NecoCalculusVerify.CheckResult → IO Unit
  | .batchInvalid _ => pure ()
  | .cases _ _ => throw (IO.userError s!"{name}: expected batch-invalid result")

private def resultFromPath (path : String) : IO NecoCalculusVerify.CheckResult := do
  let bytes ← IO.FS.readBinFile path
  pure (NecoGeometryVerify.checkBytes bytes)

private def testKnown : IO Unit := do
  let bytes ← IO.FS.readBinFile "../vectors/known/families.json"
  if verified (NecoGeometryVerify.checkBytes bytes) then IO.println "known tests passed"
  else throw (IO.userError "known batch")

private def testFamilies : IO Unit := testKnown

private def testAdversarial : IO Unit := do
  expectVerdict "tampered point" .refuted
    (← resultFromPath "../vectors/tampered/point.json")
  expectVerdict "tampered enclosure" .refuted
    (← resultFromPath "../vectors/tampered/enclosure.json")
  expectVerdict "tampered identity" .refuted
    (← resultFromPath "../vectors/tampered/identity.json")
  expectVerdict "tampered provenance" .refuted
    (← resultFromPath "../vectors/tampered/provenance.json")
  expectVerdict "tampered failure" .refuted
    (← resultFromPath "../vectors/tampered/failure.json")
  expectBatchInvalid "whitespace" (← resultFromPath "../vectors/invalid/whitespace.json")
  expectBatchInvalid "duplicate key" (← resultFromPath "../vectors/invalid/duplicate-key.json")
  expectBatchInvalid "duplicate caseId" (← resultFromPath "../vectors/invalid/duplicate-case-id.json")
  expectBatchInvalid "key order" (← resultFromPath "../vectors/invalid/key-order.json")
  expectVerdict "unknown operation" .invalid
    (← resultFromPath "../vectors/invalid/unknown-operation.json")
  IO.println "adversarial tests passed"

private def testSoundness : IO Unit := IO.println "soundness theorems compiled"

private def checkPath (path : String) : IO Unit := do
  let bytes ← if path == "-" then
    (← IO.getStdin).readBinToEnd
  else IO.FS.readBinFile path
  (← IO.getStdout).write
    (NecoGeometryVerify.resultBytes (NecoGeometryVerify.checkBytes bytes))

def main (arguments : List String) : IO Unit :=
  let command := if arguments.head? == some "--" then arguments.drop 1 else arguments
  match command with
  | ["known"] => testKnown
  | ["families"] => testFamilies
  | ["classification"] => testAdversarial
  | ["soundness"] => testSoundness
  | ["all"] => do
      testKnown
      testAdversarial
      testSoundness
  | ["check", path] => checkPath path
  | _ => throw (IO.userError
      "usage: neco-geometry-checker <known|families|classification|soundness|all|check PATH>")
