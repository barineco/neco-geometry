# neco-geometry-foundation

[日本語](README-ja.md)

`neco-geometry-foundation` defines foundational geometric types with real algebraic numbers as coordinates and strictly validates the invariants of each type. Directions, orthogonal matrices, unit quaternions, and chart declarations can be constructed only when their invariants hold, so every constructed value satisfies the invariants of its type.

## Features

**Exact Geometric Primitives**

Defines 2D and 3D points and vectors as one type per dimension. Coordinates are real algebraic numbers, and irrational coordinates such as √2 are held without approximation.

* Points: `Point2`, `Point3`
* Vectors: `Vector2`, `Vector3`

**Geometric Transformations and Rotations**

Defines types that represent directions, orthogonal matrices, unit quaternions, and transformations.
A transformation pairs an orthogonal matrix with a translation vector, and a 3D transformation also holds a unit quaternion that represents the rotation.
The invariants of the orthogonal matrix and the unit quaternion are validated when their own types are constructed.
A 3D transformation holds the orthogonal matrix and the unit quaternion directly as public fields, and the caller that constructs it must make sure that both represent the same rotation.

**Validated Chart Declarations**

A rational chart is a map that gives 2D or 3D coordinates as rational functions of one parameter. A rational chart declaration holds a string that identifies the parameter domain of the chart (`ChartDomainId`) and the display name as separate values.
Two charts can be composed when their domain identifiers match. The display name is used only for display, and whether charts can be composed depends only on the domain identifiers.

## Invariants and Validation Rules

Invariants are checked when values are constructed and when charts are composed. Norms and inner products are computed with real algebraic numbers, so the "exactly 1" and "exactly orthogonal" checks in the table compare exact values.

| Target | Invariant | Behavior on Violation |
| --- | --- | --- |
| **Direction** | The source vector is nonzero | Rejected at construction (`GeometryFailure`) |
| **Orthogonal Matrix** | Each column has norm exactly 1, and distinct columns are exactly orthogonal | Rejected at construction (`GeometryFailure`) |
| **Quaternion** | The norm is exactly 1 (w² + x² + y² + z² = 1) | Rejected at construction (`GeometryFailure`) |
| **Chart Declaration** | The domain identifier and display name have at least one character, and the denominator is nonzero | Rejected at construction (`GeometryFailure`) |
| **Chart Composition** | The domain identifiers (`ChartDomainId`) of the two charts match | Returns `GeometryFailure::ChartDomainMismatch` |

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-geometry-foundation = "0.1"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library. Returned values and failures have the same meaning as in the default configuration.

```toml
[dependencies]
neco-geometry-foundation = { version = "0.1", default-features = false }
```

## Dependencies

* [`neco-algnum`](https://docs.rs/neco-algnum): Exact coordinate arithmetic using real algebraic numbers.

## License

MIT License (see the repository [LICENSE](../LICENSE) for details)
