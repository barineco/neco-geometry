import NecoLean.Json

set_option autoImplicit false

namespace NecoGeometryVerify

open NecoLean.Json

private structure Cursor where
  remaining : List UInt8
  position : Nat

private def isSpace (byte : UInt8) : Bool :=
  byte == 32 || byte == 9 || byte == 10 || byte == 13

private def skipSpace (cursor : Cursor) : Cursor :=
  let rec loop (remaining : List UInt8) (position : Nat) : Cursor :=
    match remaining with
    | byte :: rest =>
        if isSpace byte then loop rest (position + 1) else { remaining, position }
    | [] => { remaining, position }
  loop cursor.remaining cursor.position

private def peek (cursor : Cursor) : Option UInt8 :=
  cursor.remaining.head?

private def advance (cursor : Cursor) : Option (UInt8 × Cursor) :=
  match cursor.remaining with
  | byte :: rest => some (byte, { remaining := rest, position := cursor.position + 1 })
  | [] => none

private def takeDigits (remaining : List UInt8) (count : Nat) : Nat × List UInt8 :=
  match remaining with
  | byte :: rest =>
      if 48 ≤ byte && byte ≤ 57 then takeDigits rest (count + 1) else (count, remaining)
  | [] => (count, [])

private def parseToken (cursor : Cursor) (token : List UInt8) (value : Value) :
    Option (Value × Cursor) :=
  let rec loop (remaining : List UInt8) (state : Cursor) : Option (Value × Cursor) :=
    match remaining with
    | [] => some (value, state)
    | expected :: rest =>
        match advance state with
        | none => none
        | some (actual, next) =>
            if actual == expected then loop rest next else none
  loop token cursor

private def hexValue (byte : UInt8) : Option Nat :=
  if byte ≥ 48 && byte ≤ 57 then some (byte.toNat - 48)
  else if byte ≥ 65 && byte ≤ 70 then some (byte.toNat - 55)
  else if byte ≥ 97 && byte ≤ 102 then some (byte.toNat - 87)
  else none

private def parseUnicodeEscape (digits : List UInt8) : Option Nat :=
  if digits.length != 4 then none
  else
    match digits.mapM hexValue with
    | none => none
    | some values => some (values.foldl (fun total digit => total * 16 + digit) 0)

private def utf8Width (bytes : List UInt8) : Option Nat :=
  match bytes with
  | first :: second :: third :: fourth :: _ =>
      if first ≤ 127 then some 1
      else if first ≥ 194 && first ≤ 223 && second ≥ 128 && second ≤ 191 then some 2
      else if first == 224 && second ≥ 160 && second ≤ 191 && third ≥ 128 && third ≤ 191 then some 3
      else if first ≥ 225 && first ≤ 236 && second ≥ 128 && second ≤ 191 && third ≥ 128 && third ≤ 191 then some 3
      else if first == 237 && second ≥ 128 && second ≤ 159 && third ≥ 128 && third ≤ 191 then some 3
      else if first ≥ 238 && first ≤ 239 && second ≥ 128 && second ≤ 191 && third ≥ 128 && third ≤ 191 then some 3
      else if first == 240 && second ≥ 144 && second ≤ 191 && third ≥ 128 && third ≤ 191 &&
          fourth ≥ 128 && fourth ≤ 191 then some 4
      else if first ≥ 241 && first ≤ 243 && second ≥ 128 && second ≤ 191 && third ≥ 128 &&
          third ≤ 191 && fourth ≥ 128 && fourth ≤ 191 then some 4
      else if first == 244 && second ≥ 128 && second ≤ 143 && third ≥ 128 && third ≤ 191 &&
          fourth ≥ 128 && fourth ≤ 191 then some 4
      else none
  | first :: second :: third :: _ =>
      if first ≤ 127 then some 1
      else if first ≥ 194 && first ≤ 223 && second ≥ 128 && second ≤ 191 then some 2
      else if first == 224 && second ≥ 160 && second ≤ 191 && third ≥ 128 && third ≤ 191 then some 3
      else if first ≥ 225 && first ≤ 236 && second ≥ 128 && second ≤ 191 && third ≥ 128 &&
          third ≤ 191 then some 3
      else if first == 237 && second ≥ 128 && second ≤ 159 && third ≥ 128 && third ≤ 191 then some 3
      else if first ≥ 238 && first ≤ 239 && second ≥ 128 && second ≤ 191 && third ≥ 128 &&
          third ≤ 191 then some 3
      else none
  | first :: second :: _ =>
      if first ≤ 127 then some 1
      else if first ≥ 194 && first ≤ 223 && second ≥ 128 && second ≤ 191 then some 2
      else none
  | first :: [] => if first ≤ 127 then some 1 else none
  | [] => none

private def utf8Code (width : Nat) (bytes : List UInt8) : Nat :=
  match width, bytes with
  | 1, first :: _ => first.toNat
  | 2, first :: second :: _ => (first.toNat - 192) * 64 + (second.toNat - 128)
  | 3, first :: second :: third :: _ =>
      (first.toNat - 224) * 4096 + (second.toNat - 128) * 64 + (third.toNat - 128)
  | 4, first :: second :: third :: fourth :: _ =>
      (first.toNat - 240) * 262144 + (second.toNat - 128) * 4096 + (third.toNat - 128) * 64 +
        (fourth.toNat - 128)
  | _, _ => 0

private def utf8Char (bytes : List UInt8) : Option (Char × Nat) :=
  match utf8Width bytes with
  | some width => some (Char.ofNat (utf8Code width bytes), width)
  | none => none

private def dropBytes (remaining : List UInt8) (count : Nat) : List UInt8 :=
  remaining.drop count

private partial def parseStringLoop (remaining : List UInt8) (position : Nat)
    (chars : List Char) : Option (String × Cursor) :=
  match remaining with
  | [] => none
  | byte :: rest =>
      if byte == 34 then
        some (String.ofList chars.reverse, { remaining := rest, position := position + 1 })
      else if byte < 32 then none
      else if byte == 92 then
        match rest with
        | [] => none
        | escaped :: afterEscape =>
            let nextPosition := position + 2
            if escaped == 34 then parseStringLoop afterEscape nextPosition ('"' :: chars)
            else if escaped == 92 then parseStringLoop afterEscape nextPosition ('\\' :: chars)
            else if escaped == 47 then parseStringLoop afterEscape nextPosition ('/' :: chars)
            else if escaped == 98 then parseStringLoop afterEscape nextPosition ('\x08' :: chars)
            else if escaped == 102 then parseStringLoop afterEscape nextPosition ('\x0c' :: chars)
            else if escaped == 110 then parseStringLoop afterEscape nextPosition ('\n' :: chars)
            else if escaped == 114 then parseStringLoop afterEscape nextPosition ('\r' :: chars)
            else if escaped == 116 then parseStringLoop afterEscape nextPosition ('\t' :: chars)
            else if escaped == 117 then
              match parseUnicodeEscape (afterEscape.take 4) with
              | none => none
              | some code =>
                  if code ≥ 0xD800 && code ≤ 0xDBFF then
                    match afterEscape.drop 4 with
                    | 92 :: 117 :: lowDigits =>
                        match parseUnicodeEscape (lowDigits.take 4) with
                        | some lowCode =>
                            if lowCode ≥ 0xDC00 && lowCode ≤ 0xDFFF then
                              let scalar := 0x10000 + (code - 0xD800) * 0x400 + (lowCode - 0xDC00)
                              parseStringLoop (lowDigits.drop 4) (nextPosition + 10)
                                (Char.ofNat scalar :: chars)
                            else none
                        | none => none
                    | _ => none
                  else if code ≥ 0xDC00 && code ≤ 0xDFFF then none
                  else
                    parseStringLoop (afterEscape.drop 4) (nextPosition + 4)
                      (Char.ofNat code :: chars)
            else none
      else
        match utf8Char (byte :: rest) with
        | some (character, width) =>
            parseStringLoop (dropBytes (byte :: rest) width) (position + width)
              (character :: chars)
        | none => none

private def parseString (cursor : Cursor) : Option (String × Cursor) :=
  match advance cursor with
  | some (34, state) => parseStringLoop state.remaining state.position []
  | _ => none

private def parseNumber (cursor : Cursor) : Option (Value × Cursor) :=
  let startRemaining := cursor.remaining
  let (signCount, afterSign) :=
    match cursor.remaining with
    | 45 :: rest => (1, rest)
    | rest => (0, rest)
  let (integerCount, afterInteger) :=
    match afterSign with
    | 48 :: rest => (1, rest)
    | byte :: rest =>
        if 49 ≤ byte && byte ≤ 57 then takeDigits rest 1 else (0, afterSign)
    | [] => (0, afterSign)
  if integerCount == 0 then none
  else
    let (fractionCount, afterFraction) :=
      match afterInteger with
      | 46 :: rest =>
          let result := takeDigits rest 0
          if result.1 == 0 then (1000000, afterInteger) else (result.1 + 1, result.2)
      | _ => (0, afterInteger)
    if fractionCount == 1000000 then none
    else
      let (exponentCount, _) :=
        match afterFraction with
        | exponent :: rest =>
            if exponent == 101 || exponent == 69 then
              let (expSignCount, rest') :=
                match rest with
                | sign :: tail =>
                    if sign == 43 || sign == 45 then (1, tail) else (0, rest)
                | [] => (0, [])
              let result := takeDigits rest' 0
              if result.1 == 0 then (1000000, afterFraction)
              else (1 + expSignCount + result.1, result.2)
            else (0, afterFraction)
        | [] => (0, [])
      if exponentCount == 1000000 then none
      else
        let consumed := signCount + integerCount + fractionCount + exponentCount
        let token := startRemaining.take consumed
        let text := String.ofList (token.map fun byte => Char.ofNat byte.toNat)
        match Number.create text with
        | .ok number =>
            some (.number number, {
              remaining := startRemaining.drop consumed
              position := cursor.position + consumed
            })
        | .error _ => none

mutual
  private partial def parseValue (cursor : Cursor) : Option (Value × Cursor) :=
    let current := skipSpace cursor
    match peek current with
    | none => none
    | some 110 => parseToken current [110, 117, 108, 108] .null
    | some 116 => parseToken current [116, 114, 117, 101] (.bool true)
    | some 102 => parseToken current [102, 97, 108, 115, 101] (.bool false)
    | some 34 =>
        match parseString current with
        | some (text, next) => some (.string text, next)
        | none => none
    | some 91 => parseArray current
    | some 123 => parseObject current
    | some 45 => parseNumber current
    | some byte =>
        if 48 ≤ byte && byte ≤ 57 then parseNumber current else none

  private partial def parseArray (cursor : Cursor) : Option (Value × Cursor) :=
    match advance cursor with
    | some (91, afterOpen) =>
        let afterOpen := skipSpace afterOpen
        if peek afterOpen == some 93 then
          match advance afterOpen with
          | some (_, next) => some (.array [], next)
          | none => none
        else
          let rec loop (state : Cursor) (values : List Value) : Option (Value × Cursor) :=
            match parseValue state with
            | none => none
            | some (value, afterValue) =>
                let next := skipSpace afterValue
                match peek next with
                | some 44 =>
                    match advance next with
                    | some (_, afterComma) => loop (skipSpace afterComma) (value :: values)
                    | none => none
                | some 93 =>
                    match advance next with
                    | some (_, closed) => some (.array (values.reverse ++ [value]), closed)
                    | none => none
                | _ => none
          loop afterOpen []
    | _ => none

  private partial def parseObject (cursor : Cursor) : Option (Value × Cursor) :=
    match advance cursor with
    | some (123, afterOpen) =>
        let afterOpen := skipSpace afterOpen
        if peek afterOpen == some 125 then
          match advance afterOpen with
          | some (_, next) => some (.object [], next)
          | none => none
        else
          let rec loop (state : Cursor) (fields : List (String × Value)) :
              Option (Value × Cursor) :=
            match parseString state with
            | none => none
            | some (key, afterKey) =>
                let colon := skipSpace afterKey
                match peek colon with
                | some 58 =>
                    match advance colon with
                    | none => none
                    | some (_, afterColon) =>
                        match parseValue (skipSpace afterColon) with
                        | none => none
                        | some (value, afterValue) =>
                            if fields.any (fun field => field.1 == key) then none
                            else
                              let next := skipSpace afterValue
                              match peek next with
                              | some 44 =>
                                  match advance next with
                                  | some (_, afterComma) =>
                                      loop (skipSpace afterComma) ((key, value) :: fields)
                                  | none => none
                              | some 125 =>
                                  match advance next with
                                  | some (_, closed) =>
                                      some (.object ((key, value) :: fields).reverse, closed)
                                  | none => none
                              | _ => none
                | _ => none
          loop afterOpen []
    | _ => none
end

def decodeCanonicalJson (bytes : ByteArray) : Option Value :=
  match parseValue { remaining := bytes.toList, position := 0 } with
  | some (value, cursor) =>
      let done := skipSpace cursor
      if done.remaining.isEmpty then some (canonicalize value) else none
  | none => none

end NecoGeometryVerify
