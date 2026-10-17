# neco-geometry

[日本語](README-ja.md)

`neco-geometry` is a crate for evaluation and operations on curves and surfaces with real algebraic coordinates. Evaluation results are exact values or enclosing intervals (intervals guaranteed to contain the exact value).

## Features

**Shape Declaration and Evaluation**

Evaluates declarations of curves and surfaces as exact values or as enclosing intervals.

* Curves: rational charts (maps that give coordinates as rational functions of a parameter)
* Parametric surfaces: surfaces whose coordinates are rational functions of two parameters
* Implicit surfaces: sparse multivariate polynomials

Each enclosing interval carries provenance.
The provenance consists of the owner identifier, the evaluation requirement, and the version and public API identifier of the implementation that computed it.
The evaluation requirement contains the consumer identifier, the target expressions, the decision request, and the precision. The consumer is the place that receives the result of the evaluation. The decision request specifies which exact decision (zero, equality, sign, or degeneracy) the consumer requests on which of the target expressions (two expressions for equality).

**Algebraic Region Classification**

Represents regions by sign conditions on functions and logical formulas built from them, and classifies a given point as interior, boundary, or exterior.

**Exact Intersection Construction**

Constructs intersection sets (points, curves, parts where the two shapes coincide, etc.) from two shapes and root separation settings (a refinement method such as bisection, an iteration limit, and a target interval width).
The intersection set is returned with evidence for the existence and uniqueness of the roots.

**Boundary Derivation and Split by Dimension**

The caller gives boundary candidates for each sign condition of an algebraic region.
A candidate consists of a curve given by polynomials (a restriction chart), parameter values on that curve that may be boundary points, and parameter values for probing both sides of the boundary. A candidate can also select, by index, one of the intersection sets (results of intersection construction) that the caller gives to boundary derivation.
The crate derives the boundary of the algebraic region by evaluating the function of the sign condition on the curve of each candidate.
The derived boundary is split into 0-, 1-, and 2-dimensional parts.
When a candidate selects an intersection set, the part derived from that candidate has dimension 0 if the set is points and dimension 1 if it is a curve.
A part whose dimension cannot be determined is returned with its reason.

**Ray Crossing**

Evaluates crossings of a three-dimensional ray with triangles or surfaces over the whole range where the ray parameter is positive.

* triangles
* trimmed parametric surfaces (surfaces whose parameter domain is limited by polygonal trim loops for the outer boundary and holes)
* implicit surfaces

The evaluation always returns the origin contact and the hits, and it returns the crossing parity when the result is determined, or the undetermined reason when the result is undetermined.

* Origin contact: whether the ray origin lies in the interior or on the boundary of a target
* Crossing parity: whether the number of crossings is even or odd
* Hits: the ordered sequence of hits
* Undetermined reason: the typed reason returned when the result is undetermined

**Surface Patches**

`ChartPatch` represents either:

* A full chart with an optional disk constraint.
* A trimmed declaration.

`DiskConstraint` tests exact membership in the closed unit disk.

Borrowed accessors expose:

* The chart.
* The domain.
* The disk constraint.
* The trimmed declaration.

This capability has no dependency on CAD or GPU crates.

### Numeric encoding

Converts surface patches and seams to finite f32 data and returns their rounding bounds separately. It preserves:

* Polynomial exponents.
* Domain closure.
* Outer and hole vertices.
* Disk constraints.
* Orientation.

The main APIs are:

* `encode_chart_patch`: Returns numeric patch data and rounding bounds.
* `encode_seam_association`: Encodes both seam endpoints with the same conversion.
* `EncodedSeamAssociation`: Parameter pairs and orientations in source endpoint order.
* `EncodingPrecision`: 1–256 enclosure bits and a finite, nonnegative maximum rounding error.
* `EncodedSurfacePatch`: Full or trimmed numeric patch.
* `EncodedCoefficients`: Term lists in x, y, z, denominator order.
* `EncodedDomain`: Parameter endpoints and closure flags.
* `EncodedTrimRestriction`: Outer and hole vertices.
* `EncodingBounds`: F32 format and the maximum scalar-rounding bound.
* `EncodingFailure`: Precision, range, or lower-operation failure with an input location.

Converts each rational or algebraic enclosure midpoint through f64 to f32, then measures its distance from both endpoints using exact dyadic arithmetic.
For a RealAlgebraic value whose candidate lies inside the enclosure, exact equality with the source establishes a zero bound.
For the remaining values, it rounds the larger distance upward to f64 and returns the maximum bound once in EncodingBounds.

The caller maps chart identities to its own primitive identifiers.

Consumers propagate rounding bounds through their evaluations and combine them with GPU arithmetic bounds. Encoding runs on the CPU with core and alloc.

### GPU crossing evaluation

The optional `gpu` feature evaluates ray contacts with affine planes and stereographic spheres with denominator `1 + u² + v²`.
Construct GPU data from an encoded patch and its rounding bounds, then combine numerical evaluation code, crossing code, and the caller's entry point into one WGSL program.

```text
GpuCrossingSurface::from_encoded: constructs numeric GPU data
GPU_CROSSING_WGSL: supplies crossing and contact WGSL code
NecoGpuSurface / neco_gpu_surface: surface type and function defined by numerical evaluation code
entry point: supplied by the caller
```

The caller allocates GPU resources and executes the program. Choose the WGSL query for the required result:

- `neco_gpu_surface_bounds`: Computes three-dimensional bounds for the surface over the parameter rectangle, including encoding error.
- `neco_gpu_surface_ray_excluded`: Returns `true` for certified ray exclusion. Closed contacts and numerically indeterminate cases remain candidates for contact evaluation.

- `neco_gpu_surface_contact`: Returns a residual-checked approximate point candidate after checking parameter-domain inclusion.
  It handles known double roots and original outer trim edges when the source expression matches the GPU model and the predicates establish the result.
  Indeterminate predicates remain `Unknown`.

  ```text
  Hit / Miss / Unknown / LowerEvaluationFailure
  ```

- `neco_gpu_contact_normal`: Returns an estimated world-space unit normal from the same primitive and a Hit's parameters, preserving the chart orientation. Non-Hit inputs, non-finite values, and degenerate differentials yield an unavailable normal.

  ```text
  NecoGpuContactNormal: direction, valid
  valid: 1 = available, 0 = unavailable
  ```

- `neco_gpu_contact_seam_classification`: Reads the independent seam proximity classification when a point candidate is available. `Contact` compares the distance from the candidate to a seam endpoint with the error bounds.

  ```text
  Off / Contact / Unknown
  ```

- `neco_gpu_crossing`: Uses the same point calculation and returns an overall `Unknown` whenever the seam proximity classification is other than off.

## Specifications and Constraints

Constraints are set to keep operations reliable and to limit the consumption of computational resources. Their values are the same in every execution environment.

| Operation | Constraint | Behavior on Violation |
| --- | --- | --- |
| **Intersection Construction** | Iteration limit of the root separation settings (depth of bisecting the parameter interval) ≤ 19, subintervals produced by bisection ≤ 65,535 | Returns `GeometryFailure` |
| **Boundary Derivation** | Boundary candidates ≤ 64, intersection sets ≤ 16, candidate parameter values and parameter values for probing both sides of the boundary ≤ 4,096 each per candidate | Returns `GeometryFailure` |
| **Input Validation** | Rejects invalid intervals, polynomials with duplicate or zero-coefficient terms, invalid dimensions, invalid logical formulas for regions, and invalid settings (such as root separation settings) | Rejected at construction, or returned as a typed `GeometryFailure` |
| **Surface Chart and Trim** | The denominator polynomial has at least one term; each trim loop has at least 3 vertices inside the parameter domain; self-intersecting loops and hole loops that intersect the outer loop are rejected | Returns `GeometryFailure` |
| **Ray Crossing** | The parameter range is all positive values; caller-declared limits on targets and hits; rejection of degenerate triangles (collinear vertices or non-finite vertices) and invalid trim loops | Returns a typed failure or an undetermined result |

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-geometry = "0.2"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library. Returned values and failures have the same meaning as in the default configuration.

```toml
[dependencies]
neco-geometry = { version = "0.2", default-features = false }
```

## Dependencies

* [`neco-geometry-foundation`](https://docs.rs/neco-geometry-foundation): Declarations of foundational values and charts.
* [`neco-calculus`](https://docs.rs/neco-calculus): Enclosure evaluation and evidence for the existence of roots.

## License

MIT License (see the repository [LICENSE](../LICENSE) for details)
