struct NecoGpuCrossingDomain {
    minimum: vec2f,
    maximum: vec2f,
    closed_u: vec2u,
    closed_v: vec2u,
    disk_radius: f32,
    flags: u32,
    orientation: i32,
    encoded_error_bound: f32,
}
struct NecoGpuCrossingPrimitive {
    origin_radius: vec4f,
    axis_u_mode: vec4f,
    axis_v_pad: vec4f,
    normal_mode: vec4f,
}
struct NecoGpuCrossingRay { origin: vec4f, direction: vec4f }
struct NecoGpuCrossingResult {
    parameters: vec2f,
    ray_parameter: f32,
    residual: f32,
    classification: u32,
    orientation: i32,
    lower_status: u32,
    padding: u32,
    lower_evaluation: NecoGpuSurfaceEvaluation,
}
fn neco_gpu_crossing_result(uv: vec2f, t: f32, residual: f32, classification: u32, orientation: i32, status: u32, padding: u32) -> NecoGpuCrossingResult {
    return NecoGpuCrossingResult(uv, t, residual, classification, orientation, status, padding, NecoGpuSurfaceEvaluation(vec3f(0.0), vec3f(0.0), 0.0, status));
}
struct NecoGpuTrimPolygon {
    vertices: array<vec4f, 16>,
    metadata: vec4u,
}
struct NecoGpuTrim {
    outer: NecoGpuTrimPolygon,
    metadata: vec4u,
    holes: array<NecoGpuTrimPolygon, 8>,
}
struct NecoGpuSeam { endpoint: vec4f, metadata: vec4u }
fn neco_gpu_trim_ring(ring: NecoGpuTrimPolygon, uv: vec2f, uv_error: f32) -> u32 {
    var inside = false;
    for (var index = 0u; index < ring.metadata.x; index += 1u) {
        let a = ring.vertices[index].xy;
        let b = ring.vertices[(index + 1u) % ring.metadata.x].xy;
        let edge = b - a;
        let relative = uv - a;
        let edge_length = length(edge);
        let boundary_error = uv_error + 1.0e-6 * max(1.0, edge_length);
        let cross_value = edge.x * relative.y - edge.y * relative.x;
        if abs(cross_value) <= boundary_error * edge_length
            && dot(relative, uv - b) <= boundary_error * boundary_error { return 2u; }
        if (a.y > uv.y) != (b.y > uv.y) && uv.x < (b.x - a.x) * (uv.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    return select(0u, 1u, inside);
}
fn neco_gpu_trim_classify(trim: NecoGpuTrim, uv: vec2f, uv_error: f32) -> u32 {
    let outer = neco_gpu_trim_ring(trim.outer, uv, uv_error);
    if outer != 1u { return outer; }
    for (var index = 0u; index < trim.metadata.x; index += 1u) {
        let hole = neco_gpu_trim_ring(trim.holes[index], uv, uv_error);
        if hole == 2u { return 2u; }
        if hole == 1u { return 0u; }
    }
    return 1u;
}
fn neco_gpu_seam_classify(seam: NecoGpuSeam, uv: vec2f, uv_error: f32) -> u32 {
    let distance = length(uv - seam.endpoint.xy);
    let uncertainty = uv_error + seam.endpoint.z;
    if distance <= uncertainty { return 1u; }
    if distance <= 2.0 * uncertainty { return 2u; }
    return 0u;
}
fn neco_gpu_crossing_domain(domain: NecoGpuCrossingDomain, uv: vec2f, uv_error: f32) -> u32 {
    for (var axis = 0u; axis < 2u; axis += 1u) {
        let value = uv[axis];
        let lower = domain.minimum[axis];
        let upper = domain.maximum[axis];
        if value + uv_error < lower || value - uv_error > upper { return 1u; }
        let lower_closed = select(domain.closed_v.x, domain.closed_u.x, axis == 0u);
        let upper_closed = select(domain.closed_v.y, domain.closed_u.y, axis == 0u);
        if value - uv_error <= lower {
            if uv_error == 0.0 && value == lower {
                if lower_closed == 0u { return 1u; }
                continue;
            }
            return 2u;
        }
        if value + uv_error >= upper {
            if uv_error == 0.0 && value == upper {
                if upper_closed == 0u { return 1u; }
                continue;
            }
            return 2u;
        }
    }
    if (domain.flags & 1u) != 0u {
        let radius_squared = domain.disk_radius * domain.disk_radius;
        let radius_delta = dot(uv, uv) - radius_squared;
        let disk_error = 2.0 * length(uv) * uv_error + uv_error * uv_error
            + 1.0e-6 * max(1.0, radius_squared);
        if radius_delta > disk_error { return 1u; }
        if radius_delta >= -disk_error { return 2u; }
    }
    return 0u;
}
fn neco_gpu_surface_contact(surface: NecoGpuSurface, primitive: NecoGpuCrossingPrimitive, ray: NecoGpuCrossingRay, domain: NecoGpuCrossingDomain, trim: NecoGpuTrim, seam: NecoGpuSeam) -> NecoGpuCrossingResult {
    let origin = primitive.origin_radius.xyz;
    let direction = ray.direction.xyz;
    let ray_origin = ray.origin.xyz;
    let mode = u32(primitive.axis_u_mode.w);
    var uv = vec2f(0.0);
    var t = 0.0;
    var uv_error = 0.0;
    var domain_contact_known = false;
    if !(all(abs(origin) <= vec3f(1.0e6)) && all(abs(direction) <= vec3f(1.0e4)) && all(abs(ray_origin) <= vec3f(1.0e4))) {
        return neco_gpu_crossing_result(uv, t, 0.0, 2u, 0, 0u, 0u);
    }
    if dot(direction, direction) <= 1.0e-12 { return neco_gpu_crossing_result(uv, t, 0.0, 2u, 0, 0u, 0u); }
    if mode == 0u {
        let normal = cross(primitive.axis_u_mode.xyz, primitive.axis_v_pad.xyz);
        let denominator = dot(normal, direction);
        let denominator_scale = length(normal) * length(direction);
        if denominator_scale <= 1.0e-6 || abs(denominator) <= denominator_scale * 1.0e-5 {
            return neco_gpu_crossing_result(uv, t, abs(denominator), 2u, 0, 0u, 0u);
        }
        t = dot(normal, origin - ray_origin) / denominator;
        let point = ray_origin + t * direction - origin;
        let a = dot(primitive.axis_u_mode.xyz, primitive.axis_u_mode.xyz);
        let b = dot(primitive.axis_u_mode.xyz, primitive.axis_v_pad.xyz);
        let c = dot(primitive.axis_v_pad.xyz, primitive.axis_v_pad.xyz);
        let determinant = a * c - b * b;
        if abs(determinant) <= max(a * c, 1.0) * 1.0e-6 {
            return neco_gpu_crossing_result(uv, t, determinant, 2u, 0, 0u, 0u);
        }
        uv = vec2f(c * dot(point, primitive.axis_u_mode.xyz) - b * dot(point, primitive.axis_v_pad.xyz), a * dot(point, primitive.axis_v_pad.xyz) - b * dot(point, primitive.axis_u_mode.xyz)) / determinant;
        uv_error = 1.0e-5 * (1.0 + length(point) + length(primitive.axis_u_mode.xyz) + length(primitive.axis_v_pad.xyz)) / max(abs(determinant), 1.0e-6) + domain.encoded_error_bound;
    } else {
        let radius_squared = primitive.origin_radius.w;
        let offset = ray_origin - origin;
        let a = dot(direction, direction);
        let b = 2.0 * dot(offset, direction);
        let c = dot(offset, offset) - radius_squared;
        let discriminant = b * b - 4.0 * a * c;
        let margin = max(abs(b * b) + abs(4.0 * a * c), 1.0) * 1.0e-5;
        if discriminant < -margin { return neco_gpu_crossing_result(uv, 0.0, discriminant, 1u, 0, 0u, 0u); }
        var root = 0.0;
        var root_a = a;
        var root_b = b;
        if discriminant <= margin {
            let quadratic = neco_contact_quadratic(primitive, ray);
            if !(quadratic.exact == 1u && quadratic.discriminant == 0.0
                && quadratic.a > 0.0 && quadratic.b < 0.0
                && neco_contact_exact_model(surface, primitive, domain)) {
                return neco_gpu_crossing_result(uv, 0.0, discriminant, 2u, 0, 0u, 0u);
            }
            root_a = quadratic.a;
            root_b = quadratic.b;
        } else {
            root = sqrt(discriminant);
        }
        let roots = array<f32, 2>((-root_b - root) / (2.0 * root_a), (-root_b + root) / (2.0 * root_a));
        var selected = false;
        var uncertain = false;
        for (var root_index = 0u; root_index < 2u; root_index += 1u) {
            let candidate_t = roots[root_index];
            if candidate_t < 0.0 { continue; }
            let offset_at_root = ray_origin + candidate_t * direction - origin;
            let local_normal = vec3f(
                dot(offset_at_root, primitive.axis_u_mode.xyz) / radius_squared,
                dot(offset_at_root, primitive.axis_v_pad.xyz) / radius_squared,
                dot(offset_at_root, primitive.normal_mode.xyz) / radius_squared,
            );
            let stereographic_denominator = 1.0 + local_normal.z;
            if stereographic_denominator <= 1.0e-5 {
                if stereographic_denominator == 0.0
                    || (stereographic_denominator > 0.0 && 2.0 - stereographic_denominator > 2.0 * stereographic_denominator) {
                    continue;
                }
                uncertain = true;
                continue;
            }
            let candidate_uv = local_normal.xy / stereographic_denominator;
            let candidate_uv_error = 1.0e-5 * (1.0 + length(offset_at_root) / sqrt(radius_squared)) / stereographic_denominator
                + domain.encoded_error_bound / stereographic_denominator;
            var candidate_domain = neco_gpu_crossing_domain(domain, candidate_uv, candidate_uv_error);
            var candidate_domain_known = false;
            if candidate_domain == 2u && neco_contact_exact_model(surface, primitive, domain) {
                if neco_contact_exact_sphere_parameters(primitive, ray, candidate_t, candidate_uv) {
                    candidate_domain = neco_contact_exact_domain(domain, candidate_uv);
                    candidate_domain_known = candidate_domain == 0u;
                } else if neco_contact_equator(primitive, ray, domain) {
                    candidate_domain = 0u;
                    candidate_domain_known = true;
                }
            }
            if candidate_domain == 2u { uncertain = true; continue; }
            if candidate_domain == 1u { continue; }
            if (domain.flags & 2u) != 0u {
                let candidate_trim = neco_gpu_trim_classify(trim, candidate_uv, candidate_uv_error);
                if candidate_trim == 2u { uncertain = true; continue; }
                if candidate_trim == 0u { continue; }
            }
            t = candidate_t;
            uv = candidate_uv;
            uv_error = candidate_uv_error;
            domain_contact_known = candidate_domain_known;
            selected = true;
            break;
        }
        if uncertain { return neco_gpu_crossing_result(uv, t, discriminant, 2u, 0, 0u, 0u); }
        if !selected { return neco_gpu_crossing_result(uv, t, discriminant, 1u, 0, 0u, 0u); }
    }
    if t < 0.0 { return neco_gpu_crossing_result(uv, t, 0.0, 1u, 0, 0u, 0u); }
    var domain_result = neco_gpu_crossing_domain(domain, uv, uv_error);
    if domain_contact_known { domain_result = 0u; }
    var trim_result = 1u;
    if (domain.flags & 2u) != 0u {
        trim_result = neco_gpu_trim_classify(trim, uv, uv_error);
        if mode == 0u && (domain_result == 2u || trim_result == 2u)
            && neco_contact_exact_model(surface, primitive, domain) {
            let edge_contact = neco_contact_outer_edge(primitive, ray, domain, trim);
            if edge_contact.known == 1u {
                uv = edge_contact.parameters;
                t = edge_contact.ray_parameter;
                domain_result = 0u;
                trim_result = 1u;
            }
        }
    }
    if domain_result == 1u { return neco_gpu_crossing_result(uv, t, 0.0, 1u, 0, 0u, 0u); }
    if domain_result == 2u { return neco_gpu_crossing_result(uv, t, 0.0, 2u, 0, 0u, 0u); }
    if trim_result == 0u { return neco_gpu_crossing_result(uv, t, 0.0, 1u, 0, 0u, 0u); }
    if trim_result == 2u { return neco_gpu_crossing_result(uv, t, 0.0, 2u, 0, 0u, 0u); }
    var seam_result = 0u;
    if (domain.flags & 4u) != 0u {
        seam_result = neco_gpu_seam_classify(seam, uv, uv_error);
    }
    let lower = neco_gpu_surface(surface, uv);
    if lower.status != 0u { return NecoGpuCrossingResult(uv, t, 0.0, 3u, 0, lower.status, 0u, lower); }
    let residual = length(lower.coordinates - (ray_origin + t * direction));
    let scale = 1.0 + length(lower.coordinates) + length(ray_origin) + abs(t) * length(direction) + length(origin);
    let residual_limit = length(lower.absolute_error) + 1.0e-5 * scale
        + domain.encoded_error_bound * (1.0 + length(uv) + length(primitive.axis_u_mode.xyz) + length(primitive.axis_v_pad.xyz));
    if residual > residual_limit { return neco_gpu_crossing_result(uv, t, residual, 2u, 0, 0u, 0u); }
    return NecoGpuCrossingResult(uv, t, residual, 0u, domain.orientation, 0u, seam_result, lower);
}
fn neco_gpu_contact_seam_classification(contact: NecoGpuCrossingResult) -> u32 {
    return contact.padding;
}
fn neco_gpu_crossing(surface: NecoGpuSurface, primitive: NecoGpuCrossingPrimitive, ray: NecoGpuCrossingRay, domain: NecoGpuCrossingDomain, trim: NecoGpuTrim, seam: NecoGpuSeam) -> NecoGpuCrossingResult {
    let contact = neco_gpu_surface_contact(surface, primitive, ray, domain, trim, seam);
    if contact.classification == 0u && neco_gpu_contact_seam_classification(contact) != 0u {
        return neco_gpu_crossing_result(contact.parameters, contact.ray_parameter, 0.0, 2u, bitcast<i32>(seam.metadata.x), 0u, 0u);
    }
    return contact;
}
struct NecoGpuContactNormal {
    direction: vec3f,
    valid: u32,
}
fn neco_gpu_contact_normal(primitive: NecoGpuCrossingPrimitive, contact: NecoGpuCrossingResult) -> NecoGpuContactNormal {
    let unavailable = NecoGpuContactNormal(vec3f(0.0), 0u);
    if contact.classification != 0u || contact.lower_status != 0u || contact.lower_evaluation.status != 0u
        || (contact.orientation != 1 && contact.orientation != -1)
        || !all(abs(contact.parameters) <= vec2f(1.0))
        || !all(abs(primitive.origin_radius) <= vec4f(1.0e6))
        || !all(abs(primitive.axis_u_mode) <= vec4f(1.0e6))
        || !all(abs(primitive.axis_v_pad) <= vec4f(1.0e6))
        || !all(abs(primitive.normal_mode) <= vec4f(1.0e6)) { return unavailable; }
    var tangent_u = primitive.axis_u_mode.xyz;
    var tangent_v = primitive.axis_v_pad.xyz;
    if primitive.axis_u_mode.w == 1.0 {
        let u = contact.parameters.x;
        let v = contact.parameters.y;
        tangent_u = (1.0 - u*u + v*v) * primitive.axis_u_mode.xyz
            - 2.0*u*v * primitive.axis_v_pad.xyz - 2.0*u * primitive.normal_mode.xyz;
        tangent_v = -2.0*u*v * primitive.axis_u_mode.xyz
            + (1.0 + u*u - v*v) * primitive.axis_v_pad.xyz - 2.0*v * primitive.normal_mode.xyz;
    } else if primitive.axis_u_mode.w != 0.0 { return unavailable; }
    let raw = cross(tangent_u, tangent_v);
    let scale = max(abs(raw.x), max(abs(raw.y), abs(raw.z)));
    if !(scale > 0.0 && scale <= bitcast<f32>(0x7f7fffffu)) { return unavailable; }
    let scaled = raw / scale;
    let direction = scaled * inverseSqrt(dot(scaled, scaled)) * f32(contact.orientation);
    if !all(abs(direction) <= vec3f(bitcast<f32>(0x7f7fffffu))) { return unavailable; }
    return NecoGpuContactNormal(direction, 1u);
}
