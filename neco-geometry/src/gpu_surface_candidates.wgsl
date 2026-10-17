struct NecoGpuSurfaceBounds {
    minimum: vec3f,
    maximum: vec3f,
    finite: u32,
}
struct NecoSurfaceCandidateRange { lower: f32, upper: f32, valid: u32 }
fn neco_surface_candidate_next_down(value: f32) -> f32 {
    let bits = bitcast<u32>(value);
    if (bits == 0u || bits == 0x80000000u) { return bitcast<f32>(0x80800000u); }
    if ((bits >> 23u) & 255u) == 0u { return bitcast<f32>(0x80800000u); }
    let next = select(bits - 1u, bits + 1u, (bits >> 31u) != 0u);
    if ((next >> 23u) & 255u) == 0u { return bitcast<f32>(0x80800000u); }
    return bitcast<f32>(next);
}
fn neco_surface_candidate_next_up(value: f32) -> f32 {
    let bits = bitcast<u32>(value);
    if (bits == 0u || bits == 0x80000000u) { return bitcast<f32>(0x00800000u); }
    if ((bits >> 23u) & 255u) == 0u { return bitcast<f32>(0x00800000u); }
    let next = select(bits + 1u, bits - 1u, (bits >> 31u) != 0u);
    if ((next >> 23u) & 255u) == 0u { return bitcast<f32>(0x00800000u); }
    return bitcast<f32>(next);
}
fn neco_surface_candidate_range(lower: f32, upper: f32) -> NecoSurfaceCandidateRange {
    let lb = bitcast<u32>(lower);
    let ub = bitcast<u32>(upper);
    let lf = ((lb >> 23u) & 255u) != 255u;
    let uf = ((ub >> 23u) & 255u) != 255u;
    return NecoSurfaceCandidateRange(lower, upper, select(0u, 1u, lf && uf && lower <= upper));
}
fn neco_surface_candidate_point(value: f32) -> NecoSurfaceCandidateRange {
    let bits = bitcast<u32>(value);
    if ((bits >> 23u) & 255u) == 255u { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
    if (bits & 0x7fffffffu) == 0u { return neco_surface_candidate_range(0.0, 0.0); }
    if ((bits >> 23u) & 255u) == 0u {
        return neco_surface_candidate_range(bitcast<f32>(0x80800000u), bitcast<f32>(0x00800000u));
    }
    return neco_surface_candidate_range(value, value);
}
fn neco_surface_candidate_add(a: NecoSurfaceCandidateRange, b: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    if a.valid == 0u || b.valid == 0u || max(max(abs(a.lower), abs(a.upper)), max(abs(b.lower), abs(b.upper))) > 9.223372036854776e18 { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
    let lo = a.lower + b.lower;
    let hi = a.upper + b.upper;
    return neco_surface_candidate_range(neco_surface_candidate_next_down(lo), neco_surface_candidate_next_up(hi));
}
fn neco_surface_candidate_negate(a: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    if a.valid == 0u { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
    return neco_surface_candidate_range(-a.upper, -a.lower);
}
fn neco_surface_candidate_subtract(a: NecoSurfaceCandidateRange, b: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    return neco_surface_candidate_add(a, neco_surface_candidate_negate(b));
}
fn neco_surface_candidate_multiply(a: NecoSurfaceCandidateRange, b: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    if a.valid == 0u || b.valid == 0u || max(max(abs(a.lower), abs(a.upper)), max(abs(b.lower), abs(b.upper))) > 9.223372036854776e18 { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
    let p0 = a.lower * b.lower;
    let p1 = a.lower * b.upper;
    let p2 = a.upper * b.lower;
    let p3 = a.upper * b.upper;
    let low = min(min(p0, p1), min(p2, p3));
    let high = max(max(p0, p1), max(p2, p3));
    return neco_surface_candidate_range(neco_surface_candidate_next_down(low), neco_surface_candidate_next_up(high));
}
fn neco_surface_candidate_divide(a: NecoSurfaceCandidateRange, b: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    if a.valid == 0u || b.valid == 0u || b.lower <= 0.0 && b.upper >= 0.0 || min(abs(b.lower), abs(b.upper)) < bitcast<f32>(0x00800000u) || max(abs(b.lower), abs(b.upper)) > 9.223372036854776e18 { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
    let den = array<f32, 2>(b.lower, b.upper);
    let num = array<f32, 2>(a.lower, a.upper);
    var low = bitcast<f32>(0x7f7fffffu);
    var high = -bitcast<f32>(0x7f7fffffu);
    for (var i = 0u; i < 2u; i += 1u) {
        for (var j = 0u; j < 2u; j += 1u) {
            if abs(num[i]) > abs(den[j]) * 9.223372036854776e18 { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
            let q = num[i] / den[j];
            if ((bitcast<u32>(q) >> 23u) & 255u) == 255u { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
            var qlo = q;
            var qhi = q;
            for (var step = 0u; step < 6u; step += 1u) { qlo = neco_surface_candidate_next_down(qlo); qhi = neco_surface_candidate_next_up(qhi); }
            low = min(low, qlo);
            high = max(high, qhi);
        }
    }
    return neco_surface_candidate_range(low, high);
}
fn neco_surface_candidate_square(a: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    if a.valid == 0u { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
    let product = neco_surface_candidate_multiply(a, a);
    if product.valid == 0u { return product; }
    if a.lower <= 0.0 && a.upper >= 0.0 { return neco_surface_candidate_range(0.0, max(abs(product.lower), abs(product.upper))); }
    return neco_surface_candidate_range(max(0.0, product.lower), product.upper);
}
fn neco_surface_candidate_polynomial(p: NecoGpuPolynomial, u: NecoSurfaceCandidateRange, v: NecoSurfaceCandidateRange, error: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    if p.metadata.x > 16u { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
    var total = neco_surface_candidate_point(0.0);
    for (var i = 0u; i < p.metadata.x; i += 1u) {
        let term = p.terms[i];
        if term.exponent_u > 2u || term.exponent_v > 2u { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
        var up = neco_surface_candidate_point(1.0);
        var vp = neco_surface_candidate_point(1.0);
        if term.exponent_u == 1u { up = u; }
        if term.exponent_u == 2u { up = neco_surface_candidate_square(u); }
        if term.exponent_v == 1u { vp = v; }
        if term.exponent_v == 2u { vp = neco_surface_candidate_square(v); }
        let coefficient_midpoint = neco_surface_candidate_point(term.coefficient);
        let coefficient_lower = neco_surface_candidate_subtract(coefficient_midpoint, error);
        let coefficient_upper = neco_surface_candidate_add(coefficient_midpoint, error);
        if coefficient_lower.valid == 0u || coefficient_upper.valid == 0u { return NecoSurfaceCandidateRange(0.0, 0.0, 0u); }
        let coefficient = neco_surface_candidate_range(coefficient_lower.lower, coefficient_upper.upper);
        let monomial = neco_surface_candidate_multiply(neco_surface_candidate_multiply(coefficient, up), vp);
        total = neco_surface_candidate_add(total, monomial);
    }
    return total;
}
fn neco_surface_candidate_coordinate(numerator: NecoGpuPolynomial, denominator: NecoSurfaceCandidateRange, u: NecoSurfaceCandidateRange, v: NecoSurfaceCandidateRange, error: NecoSurfaceCandidateRange) -> NecoSurfaceCandidateRange {
    return neco_surface_candidate_divide(neco_surface_candidate_polynomial(numerator, u, v, error), denominator);
}
fn neco_gpu_surface_bounds(surface: NecoGpuSurface, domain: NecoGpuCrossingDomain) -> NecoGpuSurfaceBounds {
    let error = neco_surface_candidate_point(domain.encoded_error_bound);
    if error.valid == 0u || domain.encoded_error_bound < 0.0 { return NecoGpuSurfaceBounds(vec3f(0.0), vec3f(0.0), 0u); }
    let ue = neco_surface_candidate_point(domain.minimum.x);
    let ve = neco_surface_candidate_point(domain.minimum.y);
    let uw = neco_surface_candidate_point(domain.maximum.x);
    let vw = neco_surface_candidate_point(domain.maximum.y);
    let u_lower = neco_surface_candidate_subtract(ue, error);
    let v_lower = neco_surface_candidate_subtract(ve, error);
    let u_upper = neco_surface_candidate_add(uw, error);
    let v_upper = neco_surface_candidate_add(vw, error);
    if ue.valid == 0u || ve.valid == 0u || uw.valid == 0u || vw.valid == 0u || u_lower.valid == 0u || v_lower.valid == 0u || u_upper.valid == 0u || v_upper.valid == 0u { return NecoGpuSurfaceBounds(vec3f(0.0), vec3f(0.0), 0u); }
    let u = neco_surface_candidate_range(u_lower.lower, u_upper.upper);
    let v = neco_surface_candidate_range(v_lower.lower, v_upper.upper);
    if u.valid == 0u || v.valid == 0u { return NecoGpuSurfaceBounds(vec3f(0.0), vec3f(0.0), 0u); }
    let den = neco_surface_candidate_polynomial(surface.denominator, u, v, error);
    let x = neco_surface_candidate_coordinate(surface.x, den, u, v, error);
    let y = neco_surface_candidate_coordinate(surface.y, den, u, v, error);
    let z = neco_surface_candidate_coordinate(surface.z, den, u, v, error);
    if x.valid == 0u || y.valid == 0u || z.valid == 0u { return NecoGpuSurfaceBounds(vec3f(0.0), vec3f(0.0), 0u); }
    let xyz = array<NecoSurfaceCandidateRange, 3>(x, y, z);
    var lo = vec3f(0.0);
    var hi = vec3f(0.0);
    for (var axis = 0u; axis < 3u; axis += 1u) {
        if neco_surface_candidate_point(xyz[axis].lower).valid == 0u || neco_surface_candidate_point(xyz[axis].upper).valid == 0u { return NecoGpuSurfaceBounds(vec3f(0.0), vec3f(0.0), 0u); }
        lo[axis] = xyz[axis].lower;
        hi[axis] = xyz[axis].upper;
    }
    return NecoGpuSurfaceBounds(lo, hi, 1u);
}
fn neco_gpu_surface_ray_excluded(bounds: NecoGpuSurfaceBounds, ray: NecoGpuCrossingRay) -> bool {
    if bounds.finite == 0u { return false; }
    for (var axis = 0u; axis < 3u; axis += 1u) {
        let origin_exponent = (bitcast<u32>(ray.origin[axis]) >> 23u) & 255u;
        let direction_exponent = (bitcast<u32>(ray.direction[axis]) >> 23u) & 255u;
        let lower_exponent = (bitcast<u32>(bounds.minimum[axis]) >> 23u) & 255u;
        let upper_exponent = (bitcast<u32>(bounds.maximum[axis]) >> 23u) & 255u;
        if origin_exponent == 255u || direction_exponent == 255u || lower_exponent == 255u || upper_exponent == 255u { return false; }
        if !(abs(ray.origin[axis]) <= 1.0e4 && abs(ray.direction[axis]) <= 1.0e4 && bounds.minimum[axis] <= bounds.maximum[axis]) { return false; }
    }
    if dot(ray.direction.xyz, ray.direction.xyz) <= 1.0e-12 { return false; }
    var t_lower = 0.0;
    var t_upper = bitcast<f32>(0x7f7fffffu);
    for (var axis = 0u; axis < 3u; axis += 1u) {
        let direction_bits = bitcast<u32>(ray.direction[axis]) & 0x7fffffffu;
        if direction_bits == 0u {
            if ray.origin[axis] < bounds.minimum[axis] || ray.origin[axis] > bounds.maximum[axis] { return true; }
            continue;
        }
        if direction_bits < 0x00800000u { continue; }
        let a = neco_surface_candidate_subtract(neco_surface_candidate_point(bounds.minimum[axis]), neco_surface_candidate_point(ray.origin[axis]));
        let b = neco_surface_candidate_subtract(neco_surface_candidate_point(bounds.maximum[axis]), neco_surface_candidate_point(ray.origin[axis]));
        let direction = neco_surface_candidate_point(ray.direction[axis]);
        let ta = neco_surface_candidate_divide(a, direction);
        let tb = neco_surface_candidate_divide(b, direction);
        if ta.valid == 0u || tb.valid == 0u { continue; }
        let axis_lower = min(ta.lower, tb.lower);
        let axis_upper = max(ta.upper, tb.upper);
        t_lower = max(t_lower, axis_lower);
        t_upper = min(t_upper, axis_upper);
        if t_lower > t_upper { return true; }
    }
    return false;
}
