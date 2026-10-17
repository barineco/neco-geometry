struct NecoContactScalar { value: f32, exact: u32 }
struct NecoContactVector { value: vec3f, exact: u32 }
struct NecoContactPolynomial { coefficients: array<NecoContactScalar, 6>, exact: u32 }
struct NecoContactQuadratic { a: f32, b: f32, discriminant: f32, exact: u32 }
struct NecoContactEdge { parameters: vec2f, ray_parameter: f32, known: u32 }

fn neco_contact_exact_sum(a: f32, b: f32, result: f32) -> bool {
    let aa = bitcast<u32>(a);
    let bb = bitcast<u32>(b);
    let rr = bitcast<u32>(result);
    let ea = (aa >> 23u) & 255u;
    let eb = (bb >> 23u) & 255u;
    let er = (rr >> 23u) & 255u;
    if ea == 255u || eb == 255u || er == 255u { return false; }
    let za = (aa & 0x7fffffffu) == 0u;
    let zb = (bb & 0x7fffffffu) == 0u;
    let zr = (rr & 0x7fffffffu) == 0u;
    if za { return (zb && zr) || bb == rr; }
    if zb { return aa == rr; }
    if ea == 0u || eb == 0u { return false; }
    let exponent = min(ea, eb);
    if max(ea, eb) - exponent > 7u { return false; }
    let ma = ((aa & 0x007fffffu) | 0x00800000u) << (ea - exponent);
    let mb = ((bb & 0x007fffffu) | 0x00800000u) << (eb - exponent);
    var magnitude = 0u;
    var sign = aa >> 31u;
    if (aa >> 31u) == (bb >> 31u) {
        magnitude = ma + mb;
    } else if ma >= mb {
        magnitude = ma - mb;
    } else {
        magnitude = mb - ma;
        sign = bb >> 31u;
    }
    if magnitude == 0u { return zr; }
    if er == 0u || (rr >> 31u) != sign { return false; }
    let mr = (rr & 0x007fffffu) | 0x00800000u;
    let shift = i32(er) - i32(exponent);
    if shift >= 0 {
        if shift > 31 || mr > (0xffffffffu >> u32(shift)) { return false; }
        return magnitude == (mr << u32(shift));
    }
    if shift < -31 || magnitude > (0xffffffffu >> u32(-shift)) { return false; }
    return mr == (magnitude << u32(-shift));
}

fn neco_contact_exact_product(a: f32, b: f32, result: f32) -> bool {
    let aa = bitcast<u32>(a);
    let bb = bitcast<u32>(b);
    let rr = bitcast<u32>(result);
    let ea = (aa >> 23u) & 255u;
    let eb = (bb >> 23u) & 255u;
    let er = (rr >> 23u) & 255u;
    if ea == 255u || eb == 255u || er == 255u { return false; }
    if (aa & 0x7fffffffu) == 0u || (bb & 0x7fffffffu) == 0u {
        return (rr & 0x7fffffffu) == 0u;
    }
    if ea == 0u || eb == 0u || er == 0u { return false; }
    if ((aa ^ bb) >> 31u) != (rr >> 31u) { return false; }
    let shift = i32(er) - i32(ea) - i32(eb) + 150;
    if shift != 23 && shift != 24 { return false; }
    let ma = (aa & 0x007fffffu) | 0x00800000u;
    let mb = (bb & 0x007fffffu) | 0x00800000u;
    let mr = (rr & 0x007fffffu) | 0x00800000u;
    let p0 = (ma & 65535u) * (mb & 65535u);
    let p1 = (ma >> 16u) * (mb & 65535u);
    let p2 = (ma & 65535u) * (mb >> 16u);
    let low1 = p0 + (p1 << 16u);
    let low = low1 + (p2 << 16u);
    let high = (ma >> 16u) * (mb >> 16u) + (p1 >> 16u) + (p2 >> 16u)
        + select(0u, 1u, low1 < p0) + select(0u, 1u, low < low1);
    return low == (mr << u32(shift)) && high == (mr >> (32u - u32(shift)));
}

fn neco_contact_scalar(value: f32) -> NecoContactScalar {
    return NecoContactScalar(value, select(0u, 1u, ((bitcast<u32>(value) >> 23u) & 255u) != 255u));
}
fn neco_contact_add(a: NecoContactScalar, b: NecoContactScalar) -> NecoContactScalar {
    let value = a.value + b.value;
    return NecoContactScalar(value, a.exact & b.exact & select(0u, 1u, neco_contact_exact_sum(a.value, b.value, value)));
}
fn neco_contact_negate(a: NecoContactScalar) -> NecoContactScalar {
    return NecoContactScalar(-a.value, a.exact);
}
fn neco_contact_subtract(a: NecoContactScalar, b: NecoContactScalar) -> NecoContactScalar {
    return neco_contact_add(a, neco_contact_negate(b));
}
fn neco_contact_multiply(a: NecoContactScalar, b: NecoContactScalar) -> NecoContactScalar {
    let value = a.value * b.value;
    return NecoContactScalar(value, a.exact & b.exact & select(0u, 1u, neco_contact_exact_product(a.value, b.value, value)));
}
fn neco_contact_vector(value: vec3f) -> NecoContactVector {
    return NecoContactVector(value, neco_contact_scalar(value.x).exact & neco_contact_scalar(value.y).exact & neco_contact_scalar(value.z).exact);
}
fn neco_contact_vector_subtract(a: NecoContactVector, b: NecoContactVector) -> NecoContactVector {
    let x = neco_contact_subtract(neco_contact_scalar(a.value.x), neco_contact_scalar(b.value.x));
    let y = neco_contact_subtract(neco_contact_scalar(a.value.y), neco_contact_scalar(b.value.y));
    let z = neco_contact_subtract(neco_contact_scalar(a.value.z), neco_contact_scalar(b.value.z));
    return NecoContactVector(vec3f(x.value, y.value, z.value), a.exact & b.exact & x.exact & y.exact & z.exact);
}
fn neco_contact_dot(a: NecoContactVector, b: NecoContactVector) -> NecoContactScalar {
    let x = neco_contact_multiply(neco_contact_scalar(a.value.x), neco_contact_scalar(b.value.x));
    let y = neco_contact_multiply(neco_contact_scalar(a.value.y), neco_contact_scalar(b.value.y));
    let z = neco_contact_multiply(neco_contact_scalar(a.value.z), neco_contact_scalar(b.value.z));
    let sum = neco_contact_add(neco_contact_add(x, y), z);
    return NecoContactScalar(sum.value, a.exact & b.exact & sum.exact);
}
fn neco_contact_cross(a: NecoContactVector, b: NecoContactVector) -> NecoContactVector {
    let x = neco_contact_subtract(neco_contact_multiply(neco_contact_scalar(a.value.y), neco_contact_scalar(b.value.z)), neco_contact_multiply(neco_contact_scalar(a.value.z), neco_contact_scalar(b.value.y)));
    let y = neco_contact_subtract(neco_contact_multiply(neco_contact_scalar(a.value.z), neco_contact_scalar(b.value.x)), neco_contact_multiply(neco_contact_scalar(a.value.x), neco_contact_scalar(b.value.z)));
    let z = neco_contact_subtract(neco_contact_multiply(neco_contact_scalar(a.value.x), neco_contact_scalar(b.value.y)), neco_contact_multiply(neco_contact_scalar(a.value.y), neco_contact_scalar(b.value.x)));
    return NecoContactVector(vec3f(x.value, y.value, z.value), a.exact & b.exact & x.exact & y.exact & z.exact);
}
fn neco_contact_zero(value: NecoContactScalar) -> bool {
    return value.exact == 1u && (bitcast<u32>(value.value) & 0x7fffffffu) == 0u;
}

fn neco_contact_polynomial(polynomial: NecoGpuPolynomial) -> NecoContactPolynomial {
    var result: NecoContactPolynomial;
    result.exact = 1u;
    for (var index = 0u; index < 6u; index += 1u) {
        result.coefficients[index] = neco_contact_scalar(0.0);
    }
    if polynomial.metadata.x > 16u { result.exact = 0u; return result; }
    for (var index = 0u; index < polynomial.metadata.x; index += 1u) {
        let term = polynomial.terms[index];
        var slot = 6u;
        if term.exponent_u == 0u && term.exponent_v == 0u { slot = 0u; }
        if term.exponent_u == 1u && term.exponent_v == 0u { slot = 1u; }
        if term.exponent_u == 0u && term.exponent_v == 1u { slot = 2u; }
        if term.exponent_u == 2u && term.exponent_v == 0u { slot = 3u; }
        if term.exponent_u == 1u && term.exponent_v == 1u { slot = 4u; }
        if term.exponent_u == 0u && term.exponent_v == 2u { slot = 5u; }
        if slot == 6u { result.exact = 0u; return result; }
        let sum = neco_contact_add(result.coefficients[slot], neco_contact_scalar(term.coefficient));
        result.coefficients[slot] = sum;
        result.exact &= sum.exact;
    }
    return result;
}
fn neco_contact_polynomial_matches(polynomial: NecoGpuPolynomial, expected: array<NecoContactScalar, 6>) -> bool {
    let actual = neco_contact_polynomial(polynomial);
    if actual.exact == 0u { return false; }
    for (var index = 0u; index < 6u; index += 1u) {
        if expected[index].exact == 0u || actual.coefficients[index].value != expected[index].value { return false; }
    }
    return true;
}
fn neco_contact_coordinate_matches(polynomial: NecoGpuPolynomial, center: f32, u: f32, v: f32, normal: f32, sphere: bool) -> bool {
    let zero = neco_contact_scalar(0.0);
    let c = neco_contact_scalar(center);
    let uu = neco_contact_scalar(u);
    let vv = neco_contact_scalar(v);
    if !sphere { return neco_contact_polynomial_matches(polynomial, array<NecoContactScalar, 6>(c, uu, vv, zero, zero, zero)); }
    let n = neco_contact_scalar(normal);
    let two = neco_contact_scalar(2.0);
    let square = neco_contact_subtract(c, n);
    return neco_contact_polynomial_matches(polynomial, array<NecoContactScalar, 6>(neco_contact_add(c, n), neco_contact_multiply(two, uu), neco_contact_multiply(two, vv), square, zero, square));
}
fn neco_contact_exact_model(surface: NecoGpuSurface, primitive: NecoGpuCrossingPrimitive, domain: NecoGpuCrossingDomain) -> bool {
    if (bitcast<u32>(domain.encoded_error_bound) & 0x7fffffffu) != 0u { return false; }
    let sphere = u32(primitive.axis_u_mode.w) == 1u;
    let zero = neco_contact_scalar(0.0);
    let one = neco_contact_scalar(1.0);
    let square = select(zero.value, one.value, sphere);
    if !neco_contact_polynomial_matches(surface.denominator, array<NecoContactScalar, 6>(one, zero, zero, neco_contact_scalar(square), zero, neco_contact_scalar(square))) { return false; }
    let c = primitive.origin_radius.xyz;
    let u = primitive.axis_u_mode.xyz;
    let v = primitive.axis_v_pad.xyz;
    let n = primitive.normal_mode.xyz;
    if !(neco_contact_coordinate_matches(surface.x, c.x, u.x, v.x, n.x, sphere)
        && neco_contact_coordinate_matches(surface.y, c.y, u.y, v.y, n.y, sphere)
        && neco_contact_coordinate_matches(surface.z, c.z, u.z, v.z, n.z, sphere)) { return false; }
    let uu = neco_contact_vector(u);
    let vv = neco_contact_vector(v);
    if !sphere {
        let normal = neco_contact_cross(uu, vv);
        return normal.exact == 1u && any(normal.value != vec3f(0.0));
    }
    let nn = neco_contact_vector(n);
    let lu = neco_contact_dot(uu, uu);
    let lv = neco_contact_dot(vv, vv);
    let ln = neco_contact_dot(nn, nn);
    return lu.exact == 1u && lv.exact == 1u && ln.exact == 1u && lu.value > 0.0
        && lu.value == lv.value && lu.value == ln.value && lu.value == primitive.origin_radius.w
        && neco_contact_zero(neco_contact_dot(uu, vv))
        && neco_contact_zero(neco_contact_dot(uu, nn))
        && neco_contact_zero(neco_contact_dot(vv, nn));
}
fn neco_contact_quadratic(primitive: NecoGpuCrossingPrimitive, ray: NecoGpuCrossingRay) -> NecoContactQuadratic {
    let offset = neco_contact_vector_subtract(neco_contact_vector(ray.origin.xyz), neco_contact_vector(primitive.origin_radius.xyz));
    let direction = neco_contact_vector(ray.direction.xyz);
    let a = neco_contact_dot(direction, direction);
    let b = neco_contact_multiply(neco_contact_scalar(2.0), neco_contact_dot(offset, direction));
    let c = neco_contact_subtract(neco_contact_dot(offset, offset), neco_contact_scalar(primitive.origin_radius.w));
    let discriminant = neco_contact_subtract(neco_contact_multiply(b, b), neco_contact_multiply(neco_contact_multiply(neco_contact_scalar(4.0), a), c));
    return NecoContactQuadratic(a.value, b.value, discriminant.value, a.exact & b.exact & discriminant.exact);
}
fn neco_contact_compare(a: f32, b: f32) -> i32 {
    let aa = bitcast<u32>(a);
    let bb = bitcast<u32>(b);
    if (aa & 0x7fffffffu) == 0u && (bb & 0x7fffffffu) == 0u { return 0; }
    let ka = select(aa ^ 0x80000000u, ~aa, (aa >> 31u) != 0u);
    let kb = select(bb ^ 0x80000000u, ~bb, (bb >> 31u) != 0u);
    if ka < kb { return -1; }
    if ka > kb { return 1; }
    return 0;
}
fn neco_contact_rectangular_domain(domain: NecoGpuCrossingDomain, uv: vec2f) -> u32 {
    for (var axis = 0u; axis < 2u; axis += 1u) {
        let lower = neco_contact_compare(uv[axis], domain.minimum[axis]);
        let upper = neco_contact_compare(uv[axis], domain.maximum[axis]);
        let closed = select(domain.closed_v, domain.closed_u, axis == 0u);
        if lower < 0 || upper > 0 || (lower == 0 && closed.x == 0u) || (upper == 0 && closed.y == 0u) { return 1u; }
    }
    return 0u;
}
fn neco_contact_exact_domain(domain: NecoGpuCrossingDomain, uv: vec2f) -> u32 {
    if neco_contact_rectangular_domain(domain, uv) != 0u { return 1u; }
    if (domain.flags & 1u) != 0u {
        let uu = neco_contact_multiply(neco_contact_scalar(uv.x), neco_contact_scalar(uv.x));
        let vv = neco_contact_multiply(neco_contact_scalar(uv.y), neco_contact_scalar(uv.y));
        let radius = neco_contact_scalar(domain.disk_radius);
        let delta = neco_contact_subtract(neco_contact_add(uu, vv), neco_contact_multiply(radius, radius));
        if delta.exact == 0u { return 2u; }
        if delta.value > 0.0 { return 1u; }
    }
    return 0u;
}
fn neco_contact_exact_sphere_parameters(primitive: NecoGpuCrossingPrimitive, ray: NecoGpuCrossingRay, t: f32, uv: vec2f) -> bool {
    var offset: NecoContactVector;
    offset.exact = 1u;
    for (var axis = 0u; axis < 3u; axis += 1u) {
        let value = neco_contact_subtract(neco_contact_add(neco_contact_scalar(ray.origin[axis]), neco_contact_multiply(neco_contact_scalar(t), neco_contact_scalar(ray.direction[axis]))), neco_contact_scalar(primitive.origin_radius[axis]));
        offset.value[axis] = value.value;
        offset.exact &= value.exact;
    }
    let radius_squared = neco_contact_scalar(primitive.origin_radius.w);
    if !neco_contact_zero(neco_contact_subtract(neco_contact_dot(offset, offset), radius_squared)) { return false; }
    let denominator = neco_contact_add(radius_squared, neco_contact_dot(offset, neco_contact_vector(primitive.normal_mode.xyz)));
    if denominator.exact == 0u || denominator.value <= 0.0 { return false; }
    return neco_contact_zero(neco_contact_subtract(neco_contact_multiply(neco_contact_scalar(uv.x), denominator), neco_contact_dot(offset, neco_contact_vector(primitive.axis_u_mode.xyz))))
        && neco_contact_zero(neco_contact_subtract(neco_contact_multiply(neco_contact_scalar(uv.y), denominator), neco_contact_dot(offset, neco_contact_vector(primitive.axis_v_pad.xyz))));
}
fn neco_contact_equator(primitive: NecoGpuCrossingPrimitive, ray: NecoGpuCrossingRay, domain: NecoGpuCrossingDomain) -> bool {
    if any(domain.minimum != vec2f(-1.0)) || any(domain.maximum != vec2f(1.0))
        || any(domain.closed_u != vec2u(1u)) || any(domain.closed_v != vec2u(1u)) { return false; }
    let normal = neco_contact_vector(primitive.normal_mode.xyz);
    let offset = neco_contact_vector_subtract(neco_contact_vector(ray.origin.xyz), neco_contact_vector(primitive.origin_radius.xyz));
    return neco_contact_zero(neco_contact_dot(offset, normal))
        && neco_contact_zero(neco_contact_dot(neco_contact_vector(ray.direction.xyz), normal));
}
fn neco_contact_affine_point(primitive: NecoGpuCrossingPrimitive, uv: vec2f) -> NecoContactVector {
    var result: NecoContactVector;
    result.exact = 1u;
    for (var axis = 0u; axis < 3u; axis += 1u) {
        let value = neco_contact_add(neco_contact_add(neco_contact_scalar(primitive.origin_radius[axis]), neco_contact_multiply(neco_contact_scalar(uv.x), neco_contact_scalar(primitive.axis_u_mode[axis]))), neco_contact_multiply(neco_contact_scalar(uv.y), neco_contact_scalar(primitive.axis_v_pad[axis])));
        result.value[axis] = value.value;
        result.exact &= value.exact;
    }
    return result;
}
fn neco_contact_determinant(a: f32, b: f32, c: f32, d: f32) -> vec2f {
    let ab = a * b;
    let cd = c * d;
    let value = ab - cd;
    let error = 0.00000095367431640625 * (1.0 + abs(ab) + abs(cd));
    return vec2f(value, error);
}
fn neco_contact_outer_edge(primitive: NecoGpuCrossingPrimitive, ray: NecoGpuCrossingRay, domain: NecoGpuCrossingDomain, trim: NecoGpuTrim) -> NecoContactEdge {
    var result = NecoContactEdge(vec2f(0.0), 0.0, 0u);
    if any(domain.closed_u != vec2u(1u)) || any(domain.closed_v != vec2u(1u)) { return result; }
    for (var index = 0u; index < trim.outer.metadata.x; index += 1u) {
        let a = trim.outer.vertices[index].xy;
        let b = trim.outer.vertices[(index + 1u) % trim.outer.metadata.x].xy;
        if neco_contact_rectangular_domain(domain, a) != 0u || neco_contact_rectangular_domain(domain, b) != 0u { continue; }
        let aa = neco_contact_affine_point(primitive, a);
        let bb = neco_contact_affine_point(primitive, b);
        let edge = neco_contact_vector_subtract(bb, aa);
        let offset = neco_contact_vector_subtract(neco_contact_vector(ray.origin.xyz), aa);
        let direction = neco_contact_vector(ray.direction.xyz);
        let plane = neco_contact_cross(direction, edge);
        if !neco_contact_zero(neco_contact_dot(offset, plane)) { continue; }
        for (var first = 0u; first < 3u; first += 1u) {
            let second = (first + 1u) % 3u;
            let denominator = neco_contact_determinant(direction.value[first], edge.value[second], direction.value[second], edge.value[first]);
            if abs(denominator.x) <= denominator.y { continue; }
            let numerator_t = neco_contact_determinant(offset.value[second], edge.value[first], offset.value[first], edge.value[second]);
            let numerator_edge = neco_contact_determinant(direction.value[first], offset.value[second], direction.value[second], offset.value[first]);
            let sign = select(-1.0, 1.0, denominator.x > 0.0);
            if sign * numerator_t.x <= numerator_t.y || sign * numerator_edge.x <= numerator_edge.y { continue; }
            let remaining = sign * (denominator.x - numerator_edge.x);
            let remaining_error = denominator.y + numerator_edge.y
                + 0.00000095367431640625 * (1.0 + abs(denominator.x) + abs(numerator_edge.x));
            if remaining <= remaining_error { continue; }
            let fraction = numerator_edge.x / denominator.x;
            result.parameters = a + fraction * (b - a);
            result.ray_parameter = numerator_t.x / denominator.x;
            result.known = 1u;
            return result;
        }
    }
    return result;
}
